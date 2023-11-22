use thiserror::Error;
#[derive(Error,Clone,Debug)]
pub enum GQGMCMQTTError {
    #[error("Default: {0}")]
    Default(String),
    #[error("Received request for thread exit")]
    ExitingThread
}