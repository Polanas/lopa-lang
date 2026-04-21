use rowan::{TextRange, TextSize};

use crate::ide::File;

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct VfsPath(pub String);

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
