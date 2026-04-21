pub mod base;
pub mod diagnostics;

use salsa::{Database, Setter};

use crate::ide::diagnostics::Diagnostic;
use crate::parsing::parser;
use std::fmt::Display;
use std::ops::Range;
use std::sync::{Arc, RwLock};

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
        if self.dirty {
            self.line_starts = line_starts(self.value.as_str());
            self.dirty = false;
        }
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

    pub fn line_col_by_pos(&mut self, pos: u32) -> (u32, u32) {
        let pos = pos as usize;
        self.try_recompute();
        let line = self
            .line_starts
            .partition_point(|&i| i <= pos)
            .saturating_sub(1);
        let col = pos - self.line_starts[line];
        (line as _, col as _)
    }

    pub fn pos_by_line_col(&mut self, line: u32, col: u32) -> u32 {
        let (line, col) = (line as usize, col as usize);
        self.try_recompute();
        let pos = self.line_starts.get(line).copied().unwrap_or(0);
        (pos + col) as u32
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
#[derive(Debug)]
pub struct File {
    #[returns(ref)]
    pub contents: Arc<RwLock<FileContent>>,
}

#[salsa::tracked]
fn parse(db: &dyn salsa::Database, file: File) -> Parse<'_> {
    let contents = file.contents(db).read().unwrap();
    let (node, errors) = parser::parse(contents.as_str());
    Parse::new(db, node, errors)
}

#[salsa::db]
#[derive(Default)]
pub struct RootDatabase {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for RootDatabase {}

#[derive(Default)]
pub struct Analysis {
    pub db: RootDatabase,
}

impl Analysis {
    pub fn diagnostics(&self, file: File) -> Vec<Diagnostic> {
        diagnostics::diagnostics(&self.db, file)
    }

    pub fn apply_change(&mut self, file: File) {
        //cancel any ongoing analysis
        self.db.trigger_cancellation();
        let contents = file.contents(&self.db).clone();
        file.set_contents(&mut self.db).to(contents);
    }
}

pub type TextRange = rowan::TextRange;
