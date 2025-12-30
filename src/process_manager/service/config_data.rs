use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
/// Service confguration data
pub struct ConfigData {
    /// If this piece of config data was enabled
    pub enable: bool,
    /// The path to the output configuration file
    pub path: PathBuf,
    /// Contents of the config data
    pub text: Option<String>,
    /// The source from the nix store of the configuration file
    pub source: PathBuf,
}
