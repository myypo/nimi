use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize)]
/// Service process configuration
pub struct Process {
    /// Argv used to run the service
    pub argv: Vec<String>,
}
