//! Process Manager implementation for `Nimi`
//!
//! Can take a rust represntation of some `NixOS` modular services
//! and runs them streaming logs back to the original console.

use eyre::{Context, Result};
use log::{debug, error, info};
use std::{collections::HashMap, env, sync::Arc};
use tokio::{process::Command, task::JoinSet};
use tokio_util::sync::CancellationToken;

pub mod service;
pub mod service_manager;
pub mod settings;

pub use service::Service;
pub use service_manager::ServiceManager;
pub use settings::Settings;

/// Process Manager Struct
///
/// Responsible for starting the services and streaming their outputs to the console
pub struct ProcessManager {
    services: HashMap<String, Service>,
    settings: Settings,
}

impl ProcessManager {
    /// Create a new process manager instance
    pub fn new(services: HashMap<String, Service>, settings: Settings) -> Self {
        Self { services, settings }
    }

    async fn run_startup_process(bin: &str) -> Result<()> {
        let output = Command::new(bin)
            .env_clear()
            .kill_on_drop(true)
            .output()
            .await
            .wrap_err_with(|| format!("Failed to run startup binary: {:?}", bin))?;

        debug!(target: bin, "{}", str::from_utf8(&output.stdout)?);
        let stderr = str::from_utf8(&output.stderr)?;
        if !stderr.is_empty() {
            error!(target: bin, "{}", stderr);
        }

        Ok(())
    }

    /// Spawn Child Processes
    ///
    /// Spawns every service this process manager manages into a `JoinSet`
    pub fn spawn_child_processes(
        self,
        cancel_tok: &CancellationToken,
    ) -> Result<JoinSet<Result<()>>> {
        let mut join_set = tokio::task::JoinSet::new();

        let settings = Arc::new(self.settings);
        let tmp_dir = Arc::new(env::temp_dir());

        for (name, service) in self.services {
            let cancel_tok = cancel_tok.clone();

            let settings = Arc::clone(&settings);
            let tmp_dir = Arc::clone(&tmp_dir);

            join_set.spawn(async move {
                ServiceManager::new(tmp_dir, settings, &name, service, cancel_tok)
                    .await?
                    .run()
                    .await
                    .wrap_err_with(|| format!("Process {} had an error", name))
            });
        }

        Ok(join_set)
    }

    fn spawn_shutdown_task(&self, cancel_tok: &CancellationToken) {
        let token = cancel_tok.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await?;
            token.cancel();
            Ok::<_, eyre::Report>(())
        });
    }

    /// Run the services defined for the process manager instance
    ///
    /// Terminates on `Ctrl-C`
    pub async fn run(self) -> Result<()> {
        info!("Starting process manager...");

        if let Some(startup) = &self.settings.startup.run_on_startup {
            info!("Running startup binary...");
            Self::run_startup_process(startup).await?;
        }

        let cancel_tok = CancellationToken::new();
        self.spawn_shutdown_task(&cancel_tok);

        let mut services_set = self.spawn_child_processes(&cancel_tok)?;

        while let Some(res) = services_set.join_next().await {
            let flat: Result<()> = res.map_err(Into::into).and_then(std::convert::identity);

            if let Err(e) = flat {
                cancel_tok.cancel();
                return Err(e);
            }
        }

        info!("Shutting down process manager...");

        Ok(())
    }
}
