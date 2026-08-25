//! Project configuration analysis

pub mod analyzer;
pub mod drift;
pub mod project;
pub mod secret_detector;

pub use analyzer::{ConfigAnalyzer, MissingEnvVar, UnusedConfigKey};
pub use drift::{
    ConfigDiffEntry, ConfigDiffKind, ConfigDriftReport, compare_configs, format_drift_report,
};
pub use project::{HooksConfig, RgctlConfig, RiskLevel, WatchConfig};
pub use rgctl_extraction::usage_detector::{
    ConfigConfidence, ConfigUsage, ConfigUsageDetector,
};
pub use secret_detector::{DetectedSecret, SecretDetector, Severity as SecretSeverity};
