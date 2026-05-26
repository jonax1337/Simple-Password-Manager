use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Failed to open database: {0}")]
    OpenError(String),
    #[error("Failed to save database: {0}")]
    SaveError(String),
    #[allow(dead_code)]
    #[error("Database not loaded")]
    NotLoaded,
    #[error("Invalid password or keyfile")]
    InvalidCredentials,
    #[error("Entry not found")]
    EntryNotFound,
    #[error("Group not found")]
    GroupNotFound,
    #[allow(dead_code)]
    #[error("Invalid UUID format")]
    InvalidUuid,
}
