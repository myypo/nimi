#![warn(missing_docs)]

//! [`Tini`](https://github.com/krallin/tini)-like PID 1 for containers and target for [NixOS modular services](https://nixos.org/manual/nixos/unstable/#modular-services).

use clap::Parser;
use eyre::{Context, Result};

use crate::cli::Cli;

pub mod cli;
pub mod config;
pub mod process_manager;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().wrap_err("Failed to setup color_eyre")?;
    env_logger::try_init().wrap_err("Failed to setup env_logger")?;

    Cli::parse().run().await.wrap_err("Failed to run nimi CLI")
}
