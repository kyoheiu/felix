use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum ManipulationKind {
    Delete,
    Put,
    Rename,
}

#[derive(Debug, Clone)]
pub struct Manipulation {
    pub kind: ManipulationKind,
    pub rename: Option<Renamed>,
}

#[derive(Debug, Clone)]
pub struct Renamed {
    pub original_name: PathBuf,
    pub new_name: PathBuf,
}
