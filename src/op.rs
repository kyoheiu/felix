use super::state::ItemInfo;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Operation {
    pub count: usize,
    pub op_list: Vec<OpKind>,
}

#[derive(Debug, Clone)]
pub enum OpKind {
    Delete(DeletedFiles),
    Put(PutFiles),
    Rename(RenamedFile),
}

#[derive(Debug, Clone)]
pub struct RenamedFile {
    pub original_name: PathBuf,
    pub new_name: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PutFiles {
    pub original: Vec<ItemInfo>,
    pub put: Vec<PathBuf>,
    pub dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct DeletedFiles {
    pub trash: Vec<PathBuf>,
    pub original: Vec<ItemInfo>,
    pub dir: PathBuf,
}
