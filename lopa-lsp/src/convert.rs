use notify_rust::Notification;
use tower_lsp_server::ls_types::{Position, Range};

use crate::vfs::{FileId, Vfs};

pub fn from_range(vfs: &Vfs, file: FileId, range: Range) -> std::ops::Range<usize> {
    let content = vfs.content_by_file(file);
    let mut content = content.write().unwrap();

    let start_offset =
        content.pos_by_line_col(range.start.line as _, range.start.character as _) as _;
    let end_offset = content.pos_by_line_col(range.end.line as _, range.end.character as _) as _;
    start_offset..end_offset
}

pub fn to_range(vfs: &Vfs, file: FileId, range: std::ops::Range<usize>) -> Range {
    let content = vfs.content_by_file(file);
    let mut content = content.write().unwrap();

    let (line1, col1) = content.line_col_by_pos(range.start);
    let (line2, col2) = content.line_col_by_pos(range.end);

    Range::new(
        Position::new(line1 as u32, col1 as u32),
        Position::new(line2 as u32, col2 as u32),
    )
}
