use super::state::ItemBuffer;

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
pub struct DeletedFiles {
    pub trash: Vec<ItemBuffer>,
    pub original: Vec<ItemBuffer>,
    pub dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PutFiles {
    pub original: Vec<ItemBuffer>,
    pub put: Vec<PathBuf>,
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
    match op {
        OpKind::Put(op) => {
            info!("PUT: {:?}", op.put);
        }
        OpKind::Delete(op) => {
            info!("DELETE: {:?}", item_to_pathvec(&op.original));
        }
        OpKind::Rename(op) => {
            info!("RENAME: {:?} -> {:?}", op.original_name, op.new_name);
        }
    }
}

pub fn relog(op: &OpKind, undo: bool) {
    let mut result = if undo {
        "UNDO: ".to_string()
    } else {
        "REDO: ".to_string()
    };
    match op {
        OpKind::Put(op) => {
            result.push_str("PUT");
            info!("{} {:?}", result, op.put);
        }
        OpKind::Delete(op) => {
            result.push_str("DELETE");
            info!("{} {:?}", result, item_to_pathvec(&op.original));
        }
        OpKind::Rename(op) => {
            result.push_str("RENAME");
            info!("{} {:?} -> {:?}", result, op.original_name, op.new_name);
        }
    }
}

fn item_to_pathvec(v: &Vec<ItemBuffer>) -> Vec<PathBuf> {
    let mut result = Vec::new();
    for p in v {
        result.push(p.file_path.clone());
    }
    result
}
