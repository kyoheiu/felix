use super::state::ItemInfo;
use log::info;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Operation {
    pub pos: usize,
    pub op_list: Vec<OpKind>,
}

#[derive(Debug, Clone)]
pub enum OpKind {
    Delete(DeletedFiles),
    Put(PutFiles),
    Rename(RenamedFile),
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

#[derive(Debug, Clone)]
pub struct RenamedFile {
    pub original_name: PathBuf,
    pub new_name: PathBuf,
}

impl Operation {
    /// Discard undone operations when new one is pushed.
    pub fn branch(&mut self) {
        if self.pos == 0 {
            return;
        }
        for _i in 0..self.pos {
            self.op_list.pop();
        }
    }

    pub fn push(&mut self, op: OpKind) {
        log(&op);
        self.op_list.push(op);
        self.pos = 0;
    }
}

fn log(op: &OpKind) {
    let mut result = String::new();
    match op {
        OpKind::Put(op) => {
            result.push_str("PUT: ");
            let put = path_to_string(&op.put);
            result.push_str(&put);
        }
        OpKind::Delete(op) => {
            result.push_str("DELETE: ");
            let put = item_to_string(&op.original);
            result.push_str(&put);
        }
        OpKind::Rename(op) => {
            result.push_str("RENAME: ");
            result.push_str(op.original_name.as_path().to_str().unwrap());
            result.push_str(" -> ");
            result.push_str(op.new_name.as_path().to_str().unwrap());
        }
    }
    info!("{}", result);
}

pub fn relog(op: &OpKind, undo: bool) {
    let mut result = if undo {
        "UNDO: ".to_string()
    } else {
        "REDO: ".to_string()
    };
    match op {
        OpKind::Put(_) => {
            result.push_str("PUT");
        }
        OpKind::Delete(_) => {
            result.push_str("DELETE");
        }
        OpKind::Rename(_) => {
            result.push_str("RENAME");
        }
    }
    info!("{}", result);
}

fn item_to_string(v: &Vec<ItemInfo>) -> String {
    let mut result = String::new();
    for p in v {
        result.push_str(p.file_path.as_path().to_str().unwrap());
        result.push(' ');
    }
    result
}

fn path_to_string(v: &Vec<PathBuf>) -> String {
    let mut result = String::new();
    for p in v {
        result.push_str(p.as_path().to_str().unwrap());
        result.push(' ');
    }
    result
}
