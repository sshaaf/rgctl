//! Append-only violation timeline keyed by stable entity identity and policy rule.

use crate::policy_diff::TemporalClass;
use rgctl_error::{Error, Result};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Ledger file name under `.rgctl/`.
pub const VIOLATION_LEDGER_FILE: &str = "violation_ledger.jsonl";

/// One append-only ledger record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ViolationLedgerEntry {
    /// `StableNodeKey` as `u64`.
    pub stable_key: u64,
    /// Policy rule id (e.g. `max_impact_nodes`).
    pub rule: String,
    /// Temporal class recorded for this observation.
    #[serde(rename = "class")]
    pub class: TemporalClass,
    /// Git commit SHA when the observation was recorded.
    pub commit: String,
    /// Unix timestamp string.
    pub ts: String,
    /// Optional human-readable symbol name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

/// In-memory view of the latest class per `(stable_key, rule)` plus append path.
#[derive(Debug, Default)]
pub struct ViolationLedger {
    last: HashMap<(u64, String), TemporalClass>,
    first_seen: HashMap<(u64, String), u64>,
    path: Option<PathBuf>,
}

impl ViolationLedger {
    /// Load existing ledger entries from `{rgctl_dir}/violation_ledger.jsonl`.
    pub fn open(rgctl_dir: &Path) -> Result<Self> {
        let path = rgctl_dir.join(VIOLATION_LEDGER_FILE);
        let mut ledger = Self {
            last: HashMap::new(),
            first_seen: HashMap::new(),
            path: Some(path.clone()),
        };
        if path.is_file() {
            ledger.load_from_file(&path)?;
        }
        Ok(ledger)
    }

    /// In-memory ledger without persistence (tests).
    pub fn in_memory() -> Self {
        Self {
            last: HashMap::new(),
            first_seen: HashMap::new(),
            path: None,
        }
    }

    /// Unix seconds when this `(stable_key, rule)` was first observed as a violation.
    pub fn first_seen_secs(&self, stable_key: u64, rule: &str) -> Option<u64> {
        self.first_seen
            .get(&(stable_key, rule.to_string()))
            .copied()
    }

    /// Whole-day age of a violation from ledger `first_seen` to `now_secs`.
    pub fn violation_age_days(&self, stable_key: u64, rule: &str, now_secs: u64) -> Option<u64> {
        self.first_seen_secs(stable_key, rule)
            .map(|first| now_secs.saturating_sub(first) / 86_400)
    }

    /// True when the last recorded class for this key was `resolved`.
    pub fn was_resolved(&self, stable_key: u64, rule: &str) -> bool {
        self.last
            .get(&(stable_key, rule.to_string()))
            .is_some_and(|class| *class == TemporalClass::Resolved)
    }

    /// Last recorded class for a key, if any.
    pub fn last_class(&self, stable_key: u64, rule: &str) -> Option<TemporalClass> {
        self.last.get(&(stable_key, rule.to_string())).copied()
    }

    /// Append one entry and update the in-memory last-state index.
    pub fn append(&mut self, entry: ViolationLedgerEntry) -> Result<()> {
        self.last
            .insert((entry.stable_key, entry.rule.clone()), entry.class);
        self.note_first_seen(&entry);
        if let Some(path) = &self.path {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| Error::Other(format!("open violation ledger: {e}")))?;
            let line = serde_json::to_string(&entry)
                .map_err(|e| Error::Other(format!("serialize ledger entry: {e}")))?;
            writeln!(&mut file, "{line}")
                .map_err(|e| Error::Other(format!("append violation ledger: {e}")))?;
        }
        Ok(())
    }

    fn load_from_file(&mut self, path: &Path) -> Result<()> {
        let file = File::open(path)
            .map_err(|e| Error::Other(format!("read violation ledger: {e}")))?;
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|e| Error::Other(format!("read ledger line: {e}")))?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: ViolationLedgerEntry = serde_json::from_str(&line)
                .map_err(|e| Error::Other(format!("parse ledger line: {e}")))?;
            self.last
                .insert((entry.stable_key, entry.rule.clone()), entry.class);
            self.note_first_seen(&entry);
        }
        Ok(())
    }

    fn note_first_seen(&mut self, entry: &ViolationLedgerEntry) {
        if entry.class == TemporalClass::Resolved {
            return;
        }
        let ts = entry.ts.parse::<u64>().unwrap_or(0);
        let key = (entry.stable_key, entry.rule.clone());
        match self.first_seen.get(&key) {
            Some(existing) if *existing <= ts => {}
            _ => {
                self.first_seen.insert(key, ts);
            }
        }
    }
}

/// Build a ledger entry for one temporal delta.
pub fn ledger_entry_from_delta(
    stable_key: u64,
    rule: &str,
    class: TemporalClass,
    commit: &str,
    symbol: Option<&str>,
) -> ViolationLedgerEntry {
    ViolationLedgerEntry {
        stable_key,
        rule: rule.to_string(),
        class,
        commit: commit.to_string(),
        ts: ledger_timestamp(),
        symbol: symbol.map(str::to_string),
    }
}

fn ledger_timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn ledger_round_trip_and_regression_lookup() {
        let tmp = TempDir::new().unwrap();
        let rgctl = tmp.path().join(".rgctl");
        std::fs::create_dir_all(&rgctl).unwrap();

        let mut ledger = ViolationLedger::open(&rgctl).unwrap();
        ledger
            .append(ledger_entry_from_delta(
                42,
                "max_impact_nodes",
                TemporalClass::Resolved,
                "abc123",
                Some("foo"),
            ))
            .unwrap();
        assert!(ledger.was_resolved(42, "max_impact_nodes"));
        assert!(!ledger.was_resolved(42, "centrality_alert_threshold"));

        let reloaded = ViolationLedger::open(&rgctl).unwrap();
        assert!(reloaded.was_resolved(42, "max_impact_nodes"));
        assert_eq!(
            reloaded.last_class(42, "max_impact_nodes"),
            Some(TemporalClass::Resolved)
        );
    }
}
