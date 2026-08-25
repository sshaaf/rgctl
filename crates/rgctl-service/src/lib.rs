//! Shared command execution for CLI JSON, HTTP, and MCP.
#![allow(missing_docs)]

pub mod blast_json;
pub mod check;
pub mod check_json;
pub mod command;
pub mod cpg;
pub mod error;
pub mod execute;
pub mod gql_json;
pub mod impact;
pub mod inspect_json;
pub mod metrics;
pub mod metrics_json;
pub mod policy;
pub mod query;
pub mod search;
pub mod semantic_json;
pub mod session;
pub mod slice_json;
pub mod status;

pub use command::{
    CheckArgs, Command, CommandRegistry, CpgArgs, CpgOp, DEFAULT_LIMIT, ImpactArgs, MetricsArgs,
    QueryArgs, SearchArgs, SearchScope,
};
pub use error::{Result, ServiceError};
pub use execute::{execute, execute_with_registry, pipeline_status_value};
pub use session::Session;
