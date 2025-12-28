#![warn(missing_docs)]

//! [`Tini`](https://github.com/krallin/tini)-like PID 1 for containers and target for [NixOS modular services](https://nixos.org/manual/nixos/unstable/#modular-services).

use std::path::PathBuf;

use clap::{Parser, Subcommand, command};
use eyre::{Context, Result};

use crate::process_manager::{ProcessManager, Service};

pub mod process_manager;

/// NixOS modular services runner and container init
///
/// # Examples
///
/// ## Generate a pre-configured binary from nixos modular services
///
/// ```nimi
/// pkgs.nimi.evalServicesConfig {
///   services."ghostunnel-plain-old" = {
///     imports = [ pkgs.ghostunnel.services.default ];
///     ghostunnel = {
///       listen = "0.0.0.0:443";
///       cert = "/root/service-cert.pem";
///       key = "/root/service-key.pem";
///       disableAuthentication = true;
///       target = "backend:80";
///       unsafeTarget = true;
///     };
///   };
///   services."ghostunnel-client-cert" = {
///     imports = [ pkgs.ghostunnel.services.default ];
///     ghostunnel = {
///       listen = "0.0.0.0:1443";
///       cert = "/root/service-cert.pem";
///       key = "/root/service-key.pem";
///       cacert = "/root/ca.pem";
///       target = "backend:80";
///       allowCN = [ "client" ];
///       unsafeTarget = true;
///     };
///   };
/// }
///
/// ## Interact with an existing config
/// ```bash
/// nimi validate --config ./my-config.json
/// nimi run --config ./my-config.json
/// ```
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to the json representation of nimi services to run
    ///
    /// To generate this use the `evalServicesConfig` of the nix
    /// package for nimi
    #[arg(short, long)]
    pub config: PathBuf,

    /// The subcommand to run
    #[command(subcommand)]
    pub command: Command,
}

/// The nimi subcommand to run
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Validate the nimi services config file
    Validate,

    /// Run nimi services based on the config file
    Run,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().wrap_err("Failed to setup color-eyre")?;

    let args = Args::parse();

    match args.command {
        Command::Validate => println!("validating config"),
        Command::Run => println!("running config"),
    }

    let manager = ProcessManager::new(vec![
        Service {
            name: "HTTP Server".to_string(),
            argv: vec![
                "nix".to_string(),
                "run".to_string(),
                "nixpkgs#http-server".to_string(),
            ],
            output_color: 24,
            ..Default::default()
        },
        Service {
            name: "HTTP Server 2".to_string(),
            argv: vec![
                "nix".to_string(),
                "run".to_string(),
                "nixpkgs#http-server".to_string(),
            ],
            output_color: 104,
            ..Default::default()
        },
    ]);

    manager.run().await.wrap_err("Process manager run failed")
}
