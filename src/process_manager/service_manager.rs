use std::{path::PathBuf, process::Stdio, sync::Arc};

use eyre::{Context, Result};
use log::{debug, error, info};
use tokio::{
    process::{Child, Command},
    sync::broadcast,
};

mod config_dir;
mod logger;

pub use config_dir::ConfigDir;
pub use logger::Logger;

use crate::process_manager::{Service, Settings, settings::RestartMode};

/// Responsible for the running of and managing of service state
pub struct ServiceManager<'a> {
    settings: Arc<Settings>,
    shutdown_rx: broadcast::Receiver<()>,

    name: &'a str,
    service: Service,

    current_restart_count: usize,

    config_dir: ConfigDir,
}

impl<'a> ServiceManager<'a> {
    pub async fn new(
        tmp_dir: PathBuf,
        settings: Arc<Settings>,
        name: &'a str,
        service: Service,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<Self> {
        Ok(Self {
            config_dir: ConfigDir::new(tmp_dir, &service.config_data).await?,

            settings,
            shutdown_rx,

            name,
            service,

            current_restart_count: 0,
        })
    }

    async fn create_service_child(&self) -> Result<Child> {
        Command::new(self.service.process.argv.binary())
            .args(self.service.process.argv.args())
            .env_clear()
            .env("XDG_CONFIG_HOME", &self.config_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .wrap_err_with(|| {
                format!(
                    "Failed to start process for service: {:?}",
                    self.service.process
                )
            })
    }

    pub async fn spawn_service_process(&mut self) -> Result<()> {
        let mut process = self.create_service_child().await?;

        Logger::Stdout.start(Arc::from(self.name), &mut process.stdout)?;
        Logger::Stderr.start(Arc::from(self.name), &mut process.stderr)?;

        tokio::select! {
            _ = self.shutdown_rx.recv() => {
                debug!(target: self.name, "Received shutdown signal");
                process.kill().await.wrap_err("Failed to kill service process")?;
                return Ok(());
            }
            status = process.wait() => {
                let status = status.wrap_err("Failed to get process status")?;
                eyre::ensure!(
                    status.success(),
                    "Service `{}` exited with status: {}",
                    self.name,
                    status
                );
            }
        }

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            let Err(e) = self.spawn_service_process().await else {
                return Ok(());
            };

            error!(target: self.name, "{}", e);

            match self.settings.restart.mode {
                RestartMode::Always => {
                    info!("Process {} exited, restarting (mode: always)", &self.name);
                }
                RestartMode::UpToCount => {
                    if self.current_restart_count >= self.settings.restart.count {
                        info!(
                            "Process {} exited, not restarting (mode: up-to-count {}/{})",
                            &self.name, self.current_restart_count, self.settings.restart.count
                        );
                        return Ok(());
                    }

                    self.current_restart_count += 1;

                    info!(
                        "Process {} exited, restarting (mode: up-to-count {}/{})",
                        &self.name, self.current_restart_count, self.settings.restart.count
                    );
                }
                RestartMode::Never => {
                    info!(
                        "Process {} exited, not restarting (mode: never)",
                        &self.name
                    );

                    return Ok(());
                }
            }

            tokio::select! {
                _ = tokio::time::sleep(self.settings.restart.time) => {},
                _ = self.shutdown_rx.recv() => {
                    info!("Received shutdown during restart delay for {}", self.name);
                    return Ok(());
                }
            }
        }
    }
}
