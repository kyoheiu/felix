use std::path::PathBuf;

#[derive(Debug)]
pub enum FxError {
    Io,
    GetItem,
    OpenItem,
    OpenNewWindow,
    TomlDe,
    TomlSer,
    WalkDir,
    Encode,
    PutItem(PathBuf),
    RemoveItem(PathBuf),
    TooSmallWindowSize,
    Log,
    Panic,
}

impl std::error::Error for FxError {}

impl std::fmt::Display for FxError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let printable = match self {
            FxError::Io => "Error: Io".to_owned(),
            FxError::GetItem => "Error: Cannot get item info".to_owned(),
            FxError::OpenItem => "Error: Cannot open item".to_owned(),
            FxError::OpenNewWindow => {
                "Error: Cannot open this type of item in new window".to_owned()
            }
            FxError::TomlDe => "Error: Cannot deserialize toml".to_owned(),
            FxError::TomlSer => "Error: Cannot serialize toml".to_owned(),
            FxError::WalkDir => "Error: Cannot read directory".to_owned(),
            FxError::Encode => "Error: Incorrect encoding".to_owned(),
            FxError::PutItem(s) => format!("Error: Cannot copy -> {:?}", s),
            FxError::RemoveItem(s) => format!("Error: Cannot remove -> {:?}", s),
            FxError::TooSmallWindowSize => "Error: Too small window size".to_owned(),
            FxError::Log => "Error: Cannot initialize logger".to_owned(),
            FxError::Panic => "Error: Felix panicked".to_owned(),
        };
        write!(f, "{}", printable)
    }
}

impl From<std::io::Error> for FxError {
    fn from(_err: std::io::Error) -> Self {
        FxError::Io
    }
}

impl From<toml::de::Error> for FxError {
    fn from(_err: toml::de::Error) -> Self {
        FxError::TomlDe
    }
}

impl From<toml::ser::Error> for FxError {
    fn from(_err: toml::ser::Error) -> Self {
        FxError::TomlSer
    }
}

impl From<walkdir::Error> for FxError {
    fn from(_err: walkdir::Error) -> Self {
        FxError::WalkDir
    }
}

impl From<log::SetLoggerError> for FxError {
    fn from(_err: log::SetLoggerError) -> Self {
        FxError::Log
    }
}
