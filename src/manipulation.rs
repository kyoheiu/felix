use std::path::PathBuf;

#[derive(Debug)]
pub enum ManipulationKind {
    Delete,
    Put,
    Rename,
}

#[derive(Debug)]
pub struct Manipulation {
    pub kind: ManipulationKind,
    pub rename: Option<Renamed>,
}

#[derive(Debug)]
pub struct Renamed {
    pub original_name: PathBuf,
    pub new_name: PathBuf,
}
