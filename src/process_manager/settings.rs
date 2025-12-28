use serde::{Deserialize, Serialize};

/// Settings Struct
///
/// Process manager runtime settings for configuring things like restart behaviour
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    /// The restart specific settings
    pub restart: Restart,
}

/// Restart Settings Struct
///
/// Configuration for how nimi gets restarted
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Restart {
    pub mode: RestartMode,
    pub time: usize,
    pub count: usize,
}

/// Restart Mode
///
/// Selects how the processes get restarted on failure
#[derive(Debug, Default, Serialize, Deserialize)]
pub enum RestartMode {
    #[default]
    #[serde(rename = "never")]
    Never,
    #[serde(rename = "up-to-count")]
    UpToCount,
    #[serde(rename = "always")]
    Always,
}
