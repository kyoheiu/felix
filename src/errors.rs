#[derive(Debug)]
pub enum FxError {
    Io,
    GetItem,
    OpenItem,
    TomlDe,
    TomlSer,
    WalkDir,
    UTF8,
    CopyItem,
    RenameItem,
    RemoveItem,
    TooSmallWindowSize,
}

impl std::error::Error for FxError {}

impl std::fmt::Display for FxError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let printable = match self {
            FxError::Io => "Error: Io",
            FxError::GetItem => "Error: Cannot get item info",
            FxError::OpenItem => "Error: Cannot open item",
            FxError::TomlDe => "Error: Cannot deserialize toml",
            FxError::TomlSer => "Error: Cannot serialize toml",
            FxError::WalkDir => "Error: Cannot read directory",
            FxError::UTF8 => "Error: Incorrect encoding",
            FxError::CopyItem => "Error: Cannot copy item",
            FxError::RenameItem => "Error: Cannot rename item",
            FxError::RemoveItem => "Error: Cannot remove item",
            FxError::TooSmallWindowSize => "Error: Too small window size",
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
