use eyre::{Context, Result};
use sha2::{Digest, Sha256};
use std::{ffi::OsStr, io::ErrorKind, path::PathBuf};
use tokio::fs;

use crate::process_manager::service::ConfigDataMap;

pub struct ConfigDir(PathBuf);

impl ConfigDir {
    pub async fn new(tmp_dir: PathBuf, config_data: &ConfigDataMap) -> Result<Self> {
        let dir_name = Self::generate_config_directory_name(config_data)
            .wrap_err("Failed to generate config directory name")?;

        let cfg_dir_path = tmp_dir.join(&dir_name);

        match fs::create_dir(&cfg_dir_path).await {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::AlreadyExists => return Ok(Self(cfg_dir_path)),
            Err(e) => return Err(e).wrap_err("Failed to create config dir"),
        }

        for cfg in config_data.values() {
            let out_location = cfg_dir_path.join(&cfg.path);
            fs::symlink(&cfg.source, out_location)
                .await
                .wrap_err_with(|| {
                    format!("Failed to create symlink for config file: {:?}", cfg.path)
                })?;
        }

        Ok(Self(cfg_dir_path))
    }

    pub fn generate_config_directory_name(config_data: &ConfigDataMap) -> Result<String> {
        let bytes = serde_json::to_vec(&config_data).wrap_err_with(|| {
            format!(
                "Failed to serialize config data files to bytes: {:?}",
                config_data
            )
        })?;
        let digest = Sha256::digest(&bytes);

        Ok(format!("nimi-config-{:x}", digest))
    }
}

impl AsRef<OsStr> for ConfigDir {
    fn as_ref(&self) -> &OsStr {
        self.0.as_ref()
    }
}
