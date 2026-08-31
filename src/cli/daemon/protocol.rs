//! Line-oriented JSON control protocol (Unix socket / Windows port).

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// Serialize one JSON value as a single line and write it in full (handles partial `write`s).
pub fn write_control_line<W: Write, T: Serialize>(writer: &mut W, value: &T) -> Result<()> {
    let mut line = serde_json::to_string(value).context("serialize control message")?;
    line.push('\n');
    writer
        .write_all(line.as_bytes())
        .context("write control message")?;
    writer.flush().context("flush control message")?;
    Ok(())
}

/// Read one line (until `\n` or EOF) for large JSON payloads on Unix sockets.
pub fn read_control_line<R: Read>(reader: &mut R) -> Result<String> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let n = reader
            .read(&mut chunk)
            .context("read control message")?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&chunk[..n]);
        if data.contains(&b'\n') {
            break;
        }
    }
    if data.is_empty() {
        bail!("empty control message");
    }
    String::from_utf8(data).context("decode control message")
}

#[cfg(unix)]
#[cfg(test)]
mod io_tests {
    use super::*;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::thread;

    #[test]
    fn control_line_roundtrip_exceeds_8192_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sock = dir.path().join("ctl.sock");
        let listener = UnixListener::bind(&sock).expect("bind");

        let payload = "x".repeat(20_000);
        let expect = payload.clone();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let req = read_control_line(&mut stream).expect("read req");
            assert_eq!(req.trim(), r#"{"op":"ping"}"#);
            write_control_line(
                &mut stream,
                &ControlResponse::Ok {
                    message: payload.clone(),
                },
            )
            .expect("write resp");
        });

        let mut client = UnixStream::connect(&sock).expect("connect");
        write_control_line(&mut client, &ControlRequest::Ping).expect("write req");
        let resp = read_control_line(&mut client).expect("read resp");
        handle.join().expect("server thread");
        assert!(resp.len() > 8192);
        assert!(resp.contains(&expect));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ControlRequest {
    Ping,
    Shutdown,
    Status,
    List,
    Discover(DiscoverRequest),
    Gql {
        repo: String,
        query: String,
        #[serde(default)]
        explain: bool,
        #[serde(default)]
        macro_name: Option<String>,
    },
    Impact {
        repo: String,
        symbol: String,
        #[serde(default)]
        depth: Option<usize>,
        #[serde(default)]
        class: Option<String>,
        #[serde(default)]
        file: Option<String>,
    },
    Metrics {
        repo: String,
        #[serde(default)]
        pagerank: bool,
        #[serde(default)]
        betweenness: bool,
        #[serde(default)]
        communities: bool,
    },
    Check {
        repo: String,
        policy_file: String,
    },
    McpCall {
        repo: String,
        tool: String,
        arguments: serde_json::Value,
    },
    McpRpc {
        message: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverRequest {
    pub source: String,
    #[serde(default)]
    pub languages: Option<String>,
    #[serde(default)]
    pub exclude: Option<String>,
    #[serde(default)]
    pub with_security: bool,
    #[serde(default)]
    pub with_cfg: bool,
    #[serde(default)]
    pub with_taint: bool,
    #[serde(default)]
    pub with_dfg_loops: bool,
    #[serde(default)]
    pub with_ast_skeleton: bool,
    #[serde(default)]
    pub write_json_graph: bool,
    #[serde(default)]
    pub with_dashboard: bool,
    #[serde(default)]
    pub export_migration_hints: bool,
    #[serde(default)]
    pub with_harmonic: bool,
    #[serde(default)]
    pub with_kantra: bool,
    #[serde(default)]
    pub kantra_rules: Option<String>,
    #[serde(default)]
    pub kantra_catalog: Option<String>,
    #[serde(default)]
    pub kantra_target: Option<String>,
    #[serde(default)]
    pub kantra_index_only: bool,
    #[serde(default)]
    pub full: bool,
    #[serde(default)]
    pub migration_preset: String,
    #[serde(default)]
    pub migration_order: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoListItem {
    pub name: String,
    pub source: String,
    pub status: String,
}

pub type RepoListEntry = RepoListItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "ty", rename_all = "snake_case")]
pub enum ControlResponse {
    Pong,
    Ok { message: String },
    Json { value: serde_json::Value },
    Err { message: String },
    Status {
        pid: u32,
        http: String,
        mcp: String,
        repos: usize,
    },
    List { repos: Vec<RepoListItem> },
}
