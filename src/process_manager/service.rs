use eyre::{Context, ContextCompat, Result, eyre};
use log::{debug, error};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, env, path::PathBuf, process::Stdio};
use tokio::{
    fs,
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::broadcast,
};

mod config_data;
mod process;

pub use config_data::ConfigData;
pub use process::Process;

/// Service Struct
///
/// Rust based mirror of the services as defined in the [NixOS Modular Services
/// Modules](https://github.com/NixOS/nixpkgs/blob/3574a048b30fdc5131af4069bd5e14980ce0a6d8/nixos/modules/system/service/portable/service.nix).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Service {
    /// Configuration files for the service
    #[serde(rename = "configData")]
    pub config_data: HashMap<String, ConfigData>,

    /// Process configuration
    pub process: Process,
}

impl Service {
    async fn create_config_directory(&self) -> Result<PathBuf> {
        let bytes = serde_json::to_vec(&self.config_data)
            .wrap_err("Failed to serialize config data files to bytes")?;
        let digest = Sha256::digest(&bytes);

        let dir_name = format!("nimi-config-{:x}", digest);
        let tmp = env::temp_dir();
        let tmp_subdir = tmp.join(&dir_name);

        if fs::try_exists(&tmp_subdir).await? {
            return Ok(tmp_subdir);
        }

        fs::create_dir(&tmp_subdir).await?;

        for cfg in self.config_data.values() {
            let out_location = tmp_subdir.join(&cfg.path);
            fs::symlink(&cfg.source, out_location)
                .await
                .wrap_err_with(|| {
                    format!("Failed to create symlink for config file: {:?}", cfg.path)
                })?;
        }

        Ok(tmp_subdir)
    }

    async fn spawn_process(&self) -> Result<Child> {
        let config_dir = self.create_config_directory().await?;

        let child = Command::new(&self.process.argv[0])
            .args(&self.process.argv[1..])
            .env_clear()
            .env("XDG_CONFIG_HOME", config_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .wrap_err_with(|| {
                format!(
                    "Failed to start process. `process.argv`: {:?}",
                    self.process.argv
                )
            })?;

        Ok(child)
    }

    /// Runs a service to completion, streaming it's logs to the console
    pub async fn run(&self, name: &str, shutdown_rx: &mut broadcast::Receiver<()>) -> Result<()> {
        if self.process.argv.is_empty() {
            return Err(eyre!(
                "You must give at least one argument to `process.argv` to run a service"
            ));
        }

        let mut process = self.spawn_process().await?;

        let stdout = process
            .stdout
            .take()
            .wrap_err("Failed to acquire service process stdout")?;
        let stderr = process
            .stderr
            .take()
            .wrap_err("Failed to acquire service process stderr")?;

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    debug!(target: name, "Received shutdown signal");
                    process.kill().await.wrap_err("Failed to kill service process")?;

                    process.wait().await?;

                    return Ok(());
                }
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => debug!(target: name, "{}", line),
                        Ok(None) => break,
                        Err(e) => {
                            error!(target: name, "{}", e);
                            break;
                        }
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => error!(target: name, "{}", line),
                        Ok(None) => break,
                        Err(e) => {
                            error!(target: name, "{}", e);
                            break;
                        }
                    }
                }
            }
        }

        let status = process.wait().await?;

        if !status.success() {
            return Err(eyre!("Service `{}` exited with status: {}", name, status));
        }

        Ok(())
    }
}
