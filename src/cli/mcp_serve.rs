//! MCP stdio entry — pipeline stays in the CLI crate.

use super::context::CliContext;
use super::discover::resolve_session_root;
use super::pipeline_session::spawn_full_pipeline;
use anyhow::Result;
use std::path::PathBuf;

/// Options for MCP mode.
pub struct McpServeArgs {
    pub path: Option<String>,
    pub no_pipeline: bool,
}

/// Run MCP on stdio. Starts the full pipeline in-process unless `--no-pipeline`.
pub fn serve(ctx: &CliContext, args: McpServeArgs) -> Result<()> {
    let root = PathBuf::from(resolve_session_root(ctx, args.path.as_deref()));
    let no_pipeline = args.no_pipeline;
    let verbose = ctx.verbose;
    let pipeline_root = root.clone();
    rgctl_mcp::serve(rgctl_mcp::McpServeArgs {
        repo: root,
        on_start: if no_pipeline {
            None
        } else {
            Some(Box::new(move || {
                let _ = spawn_full_pipeline(pipeline_root, verbose);
            }))
        },
    })
}
