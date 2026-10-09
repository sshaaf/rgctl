//! rgctl CLI entry point (`rgctl`).

use anyhow::Context;
use clap::FromArgMatches;
use rgctl::cli::{self, Cli};

fn main() -> anyhow::Result<()> {
    rgctl::init();
    // Custom root help: visual Lifecycle / Query / … groups (clap cannot heading-group subcommands).
    let cmd = cli::root_command::<Cli>();
    let matches = cmd.get_matches();
    let cli = Cli::from_arg_matches(&matches)
        .context("parse CLI arguments")?;
    cli.run()
}
