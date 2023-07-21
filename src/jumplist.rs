use std::{collections::VecDeque, path::Path, path::PathBuf};

#[derive(Debug, Default)]
pub struct JumpList {
    pub pos: usize,
    pub list: VecDeque<PathBuf>,
}

impl JumpList {
    pub fn add(&mut self, p: &Path) {
        if self.pos != 0 {
            for _i in 0..self.pos {
                self.list.pop_front();
            }
        }
        self.list.push_front(p.to_path_buf());
        self.pos = 0;
    }

    pub fn get_backward(&self) -> Option<PathBuf> {
        if self.pos >= self.list.len() - 1 {
            None
        } else {
            self.list.get(self.pos + 1).cloned()
        }
    }

    pub fn pos_backward(&mut self) {
        self.pos += 1;
    }

    pub fn get_forward(&self) -> Option<PathBuf> {
        if self.pos == 0 {
            None
        } else {
            self.list.get(self.pos - 1).cloned()
        }
    }

    pub fn pos_forward(&mut self) {
        self.pos -= 1;
    }

    pub fn remove_backward(&mut self) {
        self.list.remove(self.pos + 1);
    }

    pub fn remove_forward(&mut self) {
        self.list.remove(self.pos - 1);
    }
}
