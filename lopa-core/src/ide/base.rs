use std::path::PathBuf;

use rowan::{TextRange, TextSize};

use crate::ide::File;

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct VfsPath(pub PathBuf);

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct FilePos {
    pub file_id: File,
    pub pos: TextSize,
}

impl FilePos {
    pub fn new(file_id: File, pos: TextSize) -> Self {
        Self { file_id, pos }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct FileRange {
    pub file_id: File,
    pub range: TextRange,
}

impl FileRange {
    pub fn new(file_id: File, range: TextRange) -> Self {
        Self { file_id, range }
    }

    pub fn empty(pos: FilePos) -> Self {
        Self::new(pos.file_id, TextRange::empty(pos.pos))
    }

    pub fn span(start: FilePos, end: FilePos) -> Self {
        assert_eq!(start.file_id, end.file_id);
        Self::new(start.file_id, TextRange::new(start.pos, end.pos))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, salsa::Update)]
pub struct InFile<T> {
    pub file_id: File,
    pub value: T,
}

impl<T> InFile<T> {
    pub fn new(file_id: File, value: T) -> Self {
        Self { file_id, value }
    }

    pub fn with_value<U>(&self, value: U) -> InFile<U> {
        InFile::new(self.file_id, value)
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> InFile<U> {
        InFile {
            file_id: self.file_id,
            value: f(self.value),
        }
    }

    pub fn as_ref(&self) -> InFile<&T> {
        self.with_value(&self.value)
    }
}

impl<T: Clone> InFile<&T> {
    pub fn cloned(&self) -> InFile<T> {
        self.with_value(self.value.clone())
    }
}
