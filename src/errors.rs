use thiserror::Error;

#[derive(Error, Debug)]
pub enum FxError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),
    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),
    #[error(transparent)]
    WalkDir(#[from] walkdir::Error),
    #[error("{msg}")]
    UTF8 { msg: String },
    #[error("{msg}")]
    FileCopy { msg: String },
    #[error("{msg}")]
    FileRemove { msg: String },
}
