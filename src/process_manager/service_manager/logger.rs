use std::sync::Arc;

use eyre::{Context, ContextCompat, Result};
use log::{debug, error};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader, Lines};

pub enum Logger {
    Stdout,
    Stderr,
}

impl Logger {
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
