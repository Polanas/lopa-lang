use lopa_core::parsing::parser;
use std::fmt::Display;
use std::ops::{Range, RangeInclusive};

use crate::vfs::FileId;

pub struct FileContent {
    value: String,
    line_starts: Vec<usize>,
    dirty: bool,
}

impl Display for FileContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

fn line_starts(contents: &str) -> Vec<usize> {
    let mut it = contents.bytes();
    let mut line_starts = vec![0];
    let mut count = 0;
    while let Some(byte) = it.next() {
        match byte {
            b'\n' | b'\r' => {
                if byte == b'\r' {
                    //skip \n
                    it.next();
                    count += 1;
                }
                line_starts.push(count + 1);
            }
            _ => {}
        }
        count += 1;
    }
    line_starts
}

impl FileContent {
    pub fn new(contents: String) -> Self {
        Self {
            line_starts: line_starts(contents.as_str()),
            value: contents,
            dirty: false,
        }
    }

    fn try_recompute(&mut self) {
        // if self.dirty {
        self.line_starts = line_starts(self.value.as_str());
        self.dirty = false;
        // }
    }

    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }

    pub fn len(&self) -> usize {
        self.value.as_str().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, offset: usize, text: &str) {
        self.value.insert_str(offset, text);
        self.dirty = true;
    }

    pub fn delete(&mut self, range: Range<usize>) {
        self.value.replace_range(range, "");
        self.dirty = true;
    }

    pub fn replace(&mut self, range: Range<usize>, text: &str) {
        self.value.replace_range(range, text);
        self.dirty = true;
    }

    pub fn line_col_by_pos(&mut self, pos: usize) -> (usize, usize) {
        self.try_recompute();
        let line = self
            .line_starts
            .partition_point(|&i| i <= pos)
            .saturating_sub(1);
        let col = pos - self.line_starts[line];
        (line as _, col as _)
    }

    pub fn pos_by_line_col(&mut self, line: usize, col: usize) -> usize {
        self.try_recompute();
        let pos = self.line_starts.get(line).copied().unwrap_or(0);
        pos + col
    }
}

#[salsa::tracked(debug)]
pub struct Parse<'db> {
    #[tracked]
    #[returns(ref)]
    pub node: parser::Cst,
    pub errors: Vec<parser::ParseError>,
}

#[salsa::input]
pub struct Source {
    file_id: FileId,
    #[returns(ref)]
    contents: FileContent,
}

#[salsa::tracked]
fn parse(db: &dyn salsa::Database, source: Source) -> Parse<'_> {
    let (node, errors) = parser::parse(&source.contents(db).as_str());
    Parse::new(db, node, errors)
}

// #[salsa::tracked]
// pub fn parse(db: &dyn salsa::Database, source: SourceFile) -> ParseResult<'_> {
//     ParseResult::new(db, parser::parse(source.text(db).as_ref()))
// }

#[salsa::db]
#[derive(Default)]
pub struct DbImpl {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for DbImpl {}

// #[cfg(test)]
// mod test {
//     use super::FileContent;
//
//     #[test]
//     fn byte_for_line() {
//         let mut str = FileContent::new(String::from("ƒoo\nbär\r\nbaz\n\n"));
//         dbg!(&str.line_starts);
//         assert_eq!(str.byte_for_line(0), Some(0));
//         assert_eq!(str.byte_for_line(1), Some("ƒoo\n".len()));
//         assert_eq!(str.byte_for_line(2), Some("ƒoo\nbär\r\n".len()));
//         assert_eq!(str.byte_for_line(3), Some("ƒoo\nbär\r\nbaz\n".len()));
//         assert_eq!(str.byte_for_line(4), Some("ƒoo\nbär\r\nbaz\n\n".len()));
//         assert_eq!(str.byte_for_line(5), None);
//
//         let mut str = FileContent::new(String::from("sdf\n\n"));
//         assert_eq!(str.byte_for_line(2), Some(5));
//     }
//
//     #[test]
//     fn line_for_byte() {
//         let mut str = FileContent::new(String::from("abc\ndef\n\n"));
//         assert_eq!(str.len(), 9);
//         assert_eq!(str.line_for_byte(0), Some(0));
//         assert_eq!(str.line_for_byte(1), Some(0));
//         assert_eq!(str.line_for_byte(3), Some(0));
//         assert_eq!(str.line_for_byte(4), Some(1));
//         assert_eq!(str.line_for_byte(5), Some(1));
//         assert_eq!(str.line_for_byte(8), Some(2));
//         assert_eq!(str.line_for_byte(9), Some(3));
//     }
// }
