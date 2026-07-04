use std::{collections::HashMap, ops::Range, path::PathBuf, sync::Arc};

use bimap::BiMap;
use salsa::Setter;

use crate::ide;

pub struct Vfs {
    pub contents: HashMap<ide::File, FileContent>,
    pub files: BiMap<ide::File, PathBuf>,
    pub root: Option<ide::Root>,
}

impl Vfs {
    pub fn new() -> Self {
        Self {
            files: Default::default(),
            contents: Default::default(),
            root: Default::default(),
        }
    }

    pub fn file_by_path(&self, path: &PathBuf) -> Option<ide::File> {
        self.files.get_by_right(path).copied()
    }

    pub fn path_by_file(&self, file: ide::File) -> &PathBuf {
        self.files.get_by_left(&file).unwrap()
    }

    pub fn content_by_file(&self, file: ide::File) -> &FileContent {
        &self.contents[&file]
    }

    pub fn set_root(&mut self, source_root: ide::Root) {
        self.root = Some(source_root);
    }

    pub fn root(&self) -> Option<ide::Root> {
        self.root
    }

    pub fn insert_file(
        &mut self,
        path: PathBuf,
        content: String,
        db: &mut dyn salsa::Database,
    ) -> Option<ide::File> {
        let source_root = self.root()?;
        let file_content = FileContent::new(content);
        let file = ide::File::new(db, file_content.content.clone(), path.clone(), source_root);
        self.files.insert(file, path);
        self.contents.insert(file, file_content);
        Some(file)
    }

    pub fn set_content(&mut self, db: &mut dyn salsa::Database, file: ide::File, content: String) {
        let content = FileContent::new(content);
        file.set_contents(db).to(content.content.clone());
        self.contents.insert(file, content);
    }

    pub fn change_content(
        &mut self,
        db: &mut dyn salsa::Database,
        file: ide::File,
        new_text: &str,
        range: Option<Range<usize>>,
    ) {
        match range {
            Some(range) => {
                let content = self.contents.remove(&file).unwrap();
                let mut text = content.content.to_string();
                text.replace_range(range, new_text);
                let content = FileContent::new(text);
                file.set_contents(db).to(content.content.clone());
                self.contents.insert(file, content);
            }
            None => {
                self.set_content(db, file, new_text.to_string());
            }
        }
    }
}

impl Default for Vfs {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FileContent {
    content: Arc<str>,
    line_starts: Vec<usize>,
}

fn line_starts(content: &str) -> Vec<usize> {
    let mut it = content.bytes();
    let mut line_starts = vec![0];
    let mut count = 0;
    while let Some(byte) = it.next() {
        match byte {
            b'\n' | b'\r' => {
                if byte == b'\r' {
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
    pub fn new(content: String) -> Self {
        let line_starts = line_starts(&content);
        Self {
            content: Arc::from(content),
            line_starts,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.content
    }

    pub fn len(&self) -> usize {
        self.content.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn line_col_by_pos(&self, pos: u32) -> (u32, u32) {
        let pos = pos as usize;
        let line = self
            .line_starts
            .partition_point(|&i| i <= pos)
            .saturating_sub(1);
        let col = pos - self.line_starts[line];
        (line as _, col as _)
    }

    pub fn pos_by_line_col(&self, line: u32, col: u32) -> u32 {
        let (line, col) = (line as usize, col as usize);
        let pos = self.line_starts.get(line).copied().unwrap_or(0);
        (pos + col) as u32
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}
