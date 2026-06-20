use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use bimap::BiMap;
use crate::ide::{File, FileContent, SourceRoot, TextRange, base::VfsPath};


//Maps between filesystem paths and `FileId`s, store file contents
pub struct Vfs {
    pub contents: HashMap<File, Arc<RwLock<FileContent>>>,
    pub files: BiMap<File, VfsPath>,
    pub source_root: Option<SourceRoot>,
}

impl Vfs {
    pub fn new() -> Self {
        Self {
            contents: HashMap::new(),
            files: BiMap::new(),
            source_root: None,
        }
    }


    pub fn file_by_path(&self, path: &VfsPath) -> Option<File> {
        self.files.get_by_right(path).copied()
    }

    pub fn path_by_file(&self, file: File) -> &VfsPath {
        self.files.get_by_left(&file).unwrap()
    }

    pub fn content_by_file(&self, file: File) -> Arc<RwLock<FileContent>> {
        self.contents[&file].clone()
    }

    pub fn set_source_root(&mut self, source_root: SourceRoot) {
        self.source_root = Some(source_root);
    }

    pub fn insert_file(
        &mut self,
        path: VfsPath,
        content: String,
        db: &mut dyn salsa::Database,
    ) -> File {
        let file = File::new(
            db,
            Arc::new(FileContent::new(content).into()),
            path.clone(),
            self.source_root
                //TODO: figure this out
                .unwrap_or_else(|| SourceRoot::new(db, Some(vec![]), path.0.clone())),
        );
        self.files.insert(file, path);
        self.contents.insert(file, file.contents(db).clone());
        file
    }

    pub fn set_path_content(&mut self, path: VfsPath, content: String) -> File {
        match self.file_by_path(&path) {
            Some(file) => {
                let mut contents = self.contents[&file].write().unwrap();
                let len = contents.as_str().len();
                contents.replace(0..len, &content);
                file
            }
            None => {
                panic!("use insert_file first");
            }
        }
    }

    pub fn change_file_content(&mut self, file: File, new_text: &str, range: Option<TextRange>) {
        let mut content = self.contents[&file].write().unwrap();
        let len = content.as_str().len();
        match range {
            Some(range) => {
                content.replace(
                    Into::<u32>::into(range.start()) as usize
                        ..Into::<u32>::into(range.end()) as usize,
                    new_text,
                );
            }
            None => content.replace(0..len, new_text),
        }
    }


    pub fn source_root(&self) -> Option<SourceRoot> {
        self.source_root
    }
}

impl Default for Vfs {
    fn default() -> Self {
        Self::new()
    }
}
