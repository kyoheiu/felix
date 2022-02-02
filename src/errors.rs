use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    TomlDeError(#[from] toml::de::Error),
    #[error(transparent)]
    TomlSeError(#[from] toml::ser::Error),
    #[error(transparent)]
    WalkDirError(#[from] walkdir::Error),
    #[error("{msg}")]
    UTF8Error { msg: String },
}
