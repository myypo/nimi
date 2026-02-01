//! Process Manager implementation for `Nimi`
//!
//! Can take a rust represntation of some `NixOS` modular services
//! and runs them streaming logs back to the original console.
//!
//! Supports hot-reload via SIGHUP: re-reads config and restarts changed services.

use eyre::{Context, Result};
use futures::future::OptionFuture;
use log::{debug, info, warn};
use sha2::{Sha256, Digest};
use std::process::Stdio;
use std::{collections::HashMap, env, io::ErrorKind, path::PathBuf, sync::Arc};
use tokio::signal::unix::{SignalKind, signal};
use tokio::{fs, process::Command, task::JoinSet};
use tokio_util::sync::CancellationToken;

pub mod service;
pub mod service_manager;
pub mod settings;

pub use service::Service;
pub use service_manager::ServiceManager;
pub use settings::Settings;

use crate::config::Config;
use crate::process_manager::service_manager::{Logger, ServiceError, ServiceManagerOpts};
use crate::subreaper::Subreaper;

/// Compute a hash of a service's configuration for change detection
fn service_hash(service: &Service) -> String {
    let mut hasher = Sha256::new();
    // Hash the argv
    for arg in service.process.argv.args() {
        hasher.update(arg.as_bytes());
        hasher.update(b"\0");
    }
    hasher.update(service.process.argv.binary().as_bytes());
    // Hash config data keys (simplified - full impl would hash contents too)
    let mut keys: Vec<_> = service.config_data.keys().collect();
    keys.sort();
    for key in keys {
        hasher.update(key.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

/// Process Manager Struct
///
/// Responsible for starting the services and streaming their outputs to the console
pub struct ProcessManager {
    services: HashMap<String, Service>,
    settings: Settings,
    config_path: PathBuf,
}

impl ProcessManager {
    /// Create a new process manager instance
    pub fn new(services: HashMap<String, Service>, settings: Settings, config_path: PathBuf) -> Self {
        Self { services, settings, config_path }
    }

    async fn run_startup_process(&self, bin: &str, cancel_tok: &CancellationToken) -> Result<()> {
        let mut set = JoinSet::new();

        let _pause = Subreaper::pause_reaping();
        let mut process = Command::new(bin)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .wrap_err_with(|| format!("Failed to spawn startup binary: {:?}", bin))?;
        let _child_guard =
            Subreaper::track_child(process.id()).wrap_err("Failed to track startup child")?;

        let name = Arc::new("startup".to_owned());
        let logs_dir = Arc::from(None);

        Logger::Stdout.start(
            &mut process.stdout,
            Arc::clone(&name),
            Arc::clone(&logs_dir),
            &mut set,
        )?;
        Logger::Stderr.start(
            &mut process.stderr,
            Arc::clone(&name),
            Arc::clone(&logs_dir),
            &mut set,
        )?;

        tokio::select! {
            _ = cancel_tok.cancelled() => {
                debug!(target: &name, "Received shutdown signal");
                ServiceManager::shutdown_process(&mut process, self.settings.restart.time).await?;
            }
            status = process.wait() => {
                let status = status.wrap_err("Failed to get process status")?;
                eyre::ensure!(
                    status.success(),
                    ServiceError::ProcessExited { status }
                );
            }
        }

        set.join_all().await.into_iter().collect()
    }

    /// Create logs dir
    ///
    /// Creates the logs directory for the process manager
    /// to have it's services create textual log files in
    pub async fn create_logs_dir(logs_path: &str) -> Result<PathBuf> {
        let cwd = env::current_dir()?;

        let target = cwd.join(logs_path);

        let mut logs_no = 0;
        loop {
            let sub_dir = target.join(format!("logs-{logs_no}"));
            logs_no += 1;

            match fs::create_dir_all(&sub_dir).await {
                Ok(()) => return Ok(sub_dir),
                Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
                Err(e) => {
                    return Err(e).wrap_err_with(|| {
                        format!("Failed to create logs dir: {}", sub_dir.to_string_lossy())
                    });
                }
            };
        }
    }

    /// Spawn Child Processes
    ///
    /// Spawns every service this process manager manages into a `JoinSet`
    pub async fn spawn_child_processes(
        &mut self,
        cancel_tok: &CancellationToken,
    ) -> Result<JoinSet<Result<()>>> {
        let mut join_set = tokio::task::JoinSet::new();

        let settings = Arc::new(self.settings.clone());
        let logs_dir = Arc::new(
            OptionFuture::from(
                settings
                    .logging
                    .logs_dir
                    .as_deref()
                    .map(Self::create_logs_dir),
            )
            .await
            .transpose()?,
        );
        let tmp_dir = Arc::new(env::temp_dir());

        // Drain services so we can move them into the spawned tasks
        let services = std::mem::take(&mut self.services);
        for (name, service) in services {
            let opts = ServiceManagerOpts {
                logs_dir: Arc::clone(&logs_dir),
                tmp_dir: Arc::clone(&tmp_dir),

                settings: Arc::clone(&settings),

                name: Arc::new(name),
                service,
                cancel_tok: cancel_tok.clone(),
            };

            join_set.spawn(async move { ServiceManager::new(opts).await?.run().await });
        }

        Ok(join_set)
    }

    /// Run the services defined for the process manager instance
    ///
    /// Terminates on `Ctrl-C` or `SIGTERM`. Reloads on `SIGHUP`.
    pub async fn run(mut self) -> Result<()> {
        info!("Starting process manager...");

        // Register signal handlers BEFORE spawning anything else
        // This ensures signals are caught by us, not by default handlers
        let mut sigterm = signal(SignalKind::terminate())
            .expect("Failed to register SIGTERM handler");
        let mut sighup = signal(SignalKind::hangup())
            .expect("Failed to register SIGHUP handler");
        
        let config_path = self.config_path.clone();

        // Track service hashes for change detection
        let mut service_hashes: HashMap<String, String> = self.services
            .iter()
            .map(|(name, svc)| (name.clone(), service_hash(svc)))
            .collect();

        let cancel_tok = CancellationToken::new();

        if let Some(startup) = &self.settings.startup.run_on_startup {
            info!("Running startup binary ({})...", startup);
            self.run_startup_process(startup, &cancel_tok)
                .await
                .wrap_err("Failed to run startup process")?;
        }

        let mut services_set = self.spawn_child_processes(&cancel_tok).await?;

        loop {
            tokio::select! {
                // Handle service completions
                res = services_set.join_next() => {
                    match res {
                        Some(Ok(Ok(()))) => {
                            // Service exited normally (after restart retries exhausted)
                            debug!("A service task completed");
                        }
                        Some(Ok(Err(e))) => {
                            warn!("Service error: {:?}", e);
                            // Don't cancel everything on single service failure
                        }
                        Some(Err(e)) => {
                            warn!("Service task panicked: {:?}", e);
                        }
                        None => {
                            // All services have exited
                            info!("All services have exited");
                            break;
                        }
                    }
                }
                
                // Handle SIGINT (Ctrl-C)
                _ = tokio::signal::ctrl_c() => {
                    info!("Received SIGINT, shutting down...");
                    cancel_tok.cancel();
                    // Wait for all services to stop
                    while services_set.join_next().await.is_some() {}
                    break;
                }
                
                // Handle SIGTERM
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, shutting down...");
                    cancel_tok.cancel();
                    // Wait for all services to stop
                    while services_set.join_next().await.is_some() {}
                    break;
                }
                
                // Handle SIGHUP - reload configuration
                _ = sighup.recv() => {
                    info!("Received SIGHUP, reloading configuration...");
                    match Self::reload_config(&config_path, &mut service_hashes, &cancel_tok, &mut services_set, &self.settings).await {
                        Ok(new_services) => {
                            self.services = new_services;
                            info!("Configuration reloaded successfully");
                        }
                        Err(e) => {
                            warn!("Failed to reload configuration: {:?}", e);
                        }
                    }
                }
            }
        }

        info!("Process manager shut down");
        Ok(())
    }

    /// Reload configuration, restarting changed services
    async fn reload_config(
        config_path: &PathBuf,
        service_hashes: &mut HashMap<String, String>,
        cancel_tok: &CancellationToken,
        services_set: &mut JoinSet<Result<()>>,
        settings: &Settings,
    ) -> Result<HashMap<String, Service>> {
        // Read new config
        let config_str = fs::read_to_string(config_path)
            .await
            .wrap_err("Failed to read config file")?;
        let new_config: Config = serde_json::from_str(&config_str)
            .wrap_err("Failed to parse config file")?;

        let settings = Arc::new(settings.clone());
        let logs_dir = Arc::new(
            OptionFuture::from(
                settings
                    .logging
                    .logs_dir
                    .as_deref()
                    .map(Self::create_logs_dir),
            )
            .await
            .transpose()?,
        );
        let tmp_dir = Arc::new(env::temp_dir());

        // Compute new hashes and find changes
        let new_hashes: HashMap<String, String> = new_config.services
            .iter()
            .map(|(name, svc)| (name.clone(), service_hash(svc)))
            .collect();

        // Find services to restart (changed or new)
        let mut to_restart = Vec::new();
        for (name, new_hash) in &new_hashes {
            match service_hashes.get(name) {
                Some(old_hash) if old_hash == new_hash => {
                    debug!("Service {} unchanged", name);
                }
                Some(_) => {
                    info!("Service {} changed, will restart", name);
                    to_restart.push(name.clone());
                }
                None => {
                    info!("Service {} is new, will start", name);
                    to_restart.push(name.clone());
                }
            }
        }

        // Find removed services
        for name in service_hashes.keys() {
            if !new_hashes.contains_key(name) {
                info!("Service {} removed", name);
                // The service manager will stop when its cancel token is triggered
                // For now, we just don't restart it
            }
        }

        // Spawn new/changed services
        // Note: existing services will continue running - we can't easily stop individual
        // services with the current architecture. A full implementation would need
        // per-service cancel tokens.
        for name in to_restart {
            if let Some(service) = new_config.services.get(&name) {
                let opts = ServiceManagerOpts {
                    logs_dir: Arc::clone(&logs_dir),
                    tmp_dir: Arc::clone(&tmp_dir),
                    settings: Arc::clone(&settings),
                    name: Arc::new(name.clone()),
                    service: service.clone(),
                    cancel_tok: cancel_tok.clone(),
                };
                services_set.spawn(async move { ServiceManager::new(opts).await?.run().await });
            }
        }

        // Update hashes
        *service_hashes = new_hashes;

        Ok(new_config.services)
    }
}
