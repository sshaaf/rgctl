//! rgctl CLI entry point (`rgctl`).

use clap::Parser;
use rgctl::cli::Cli;

fn main() -> anyhow::Result<()> {
    rgctl::init();
    Cli::parse().run()
}
