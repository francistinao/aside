//! One error type for every command. Serialized as a plain message for the frontend.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Store(#[from] tauri_plugin_store::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("the main thread is not available")]
    NotMainThread,
    #[error("the {0} window does not exist")]
    MissingWindow(&'static str),
    #[error("No scripts folder has been chosen yet.")]
    NoFolder,
    #[error("The scripts folder can't be found. Choose it again to continue.")]
    FolderMissing,
    #[error("That path is outside the scripts folder.")]
    OutsideVault,
    #[error("\"{0}\" is not a valid name.")]
    BadName(String),
    #[error("A script named \"{0}\" already exists.")]
    NameTaken(String),
    #[error("Couldn't move \"{0}\" to the Trash.")]
    Trash(String),
}

impl Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
