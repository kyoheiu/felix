use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("{msg}")]
    ConfigError {msg: String},
    #[error(transparent)]
    TomlError(#[from] toml::de::Error)
}