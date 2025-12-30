//! Service Manager Loggers
//!
//! Reads the logs from the sub processes and prints them from the `Nimi` instance

use std::sync::Arc;

use eyre::{Context, ContextCompat, Result};
use log::{debug, error};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader, Lines};

/// Logger type
///
/// Formats the logs differently based on if they are intended for stdout or stderr
pub enum Logger {
    /// Regular process logs
    Stdout,

    /// Process error logs
    Stderr,
}

impl Logger {
    /// Start a logger for a given file descriptor
    pub fn start<D>(self, target: Arc<str>, fd: &mut Option<D>) -> Result<()>
    where
        D: AsyncRead + Unpin + Send + 'static,
    {
        let mut reader = Self::get_lines_reader(fd)
            .wrap_err("Failed to acquire lines reader for stdout logger")?;

        tokio::spawn(async move {
            loop {
                match reader.next_line().await {
                    Ok(Some(line)) => self.log_line(&target, &line),
                    Ok(None) => break,
                    Err(e) => {
                        error!(target: &target, "{}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    fn log_line(&self, target: &str, line: &str) {
        match self {
            Self::Stdout => debug!(target: target, "{}", line),
            Self::Stderr => error!(target: target, "{}", line),
        }
    }

    fn get_lines_reader<D>(fd: &mut Option<D>) -> Result<Lines<BufReader<D>>>
    where
        D: AsyncRead,
    {
        let taken = fd.take().wrap_err("Service was missing field value")?;

        Ok(BufReader::new(taken).lines())
    }
}
