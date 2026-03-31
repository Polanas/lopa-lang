use tower_lsp_server::ls_types::{Position, Range};

use crate::vfs::{FileId, Vfs};

pub fn from_range(vfs: &Vfs, file: FileId, range: Range) -> std::ops::Range<usize> {
    let rope = vfs.content_by_file(file);
    let start_offset = rope.byte_of_line(range.start.line as _) + range.start.character as usize;
    let end_offset = rope.byte_of_line(range.end.line as _) + range.end.character as usize;
    start_offset..end_offset
}

pub fn to_range(vfs: &Vfs, file: FileId, range: std::ops::Range<usize>) -> Range {
    let rope = vfs.content_by_file(file);
    let line_start = rope.line_of_byte(range.start) as u32;
    let line_end = rope.line_of_byte(range.end) as u32;
    Range::new(
        Position::new(line_start as _, (range.start as u32) - line_start),
        Position::new(line_end as _, (range.end as u32) - line_end),
    )
}
