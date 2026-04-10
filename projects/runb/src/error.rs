use std::fmt;

#[derive(Debug)]
pub enum ContainerError {
    NotFound(String),
    AlreadyExists(String),
    InvalidState(String),
    SystemError(String),
}

impl fmt::Display for ContainerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "container not found: {id}"),
            Self::AlreadyExists(id) => write!(f, "container already exists: {id}"),
            Self::InvalidState(msg) => write!(f, "invalid state: {msg}"),
            Self::SystemError(msg) => write!(f, "system error: {msg}"),
        }
    }
}

impl std::error::Error for ContainerError {}
