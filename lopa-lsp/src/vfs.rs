use std::{
    fmt::Display,
    ops::Range,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use bimap::BiMap;
use slotmap::SlotMap;
use tower_lsp_server::ls_types::Uri;

use crate::{base::FileContent, uri_ext::UrlExt as _};

slotmap::new_key_type! { pub struct FileId; }

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct VfsPath(pub String);

//Maps between filesystem paths and `FileId`s, store file contents
pub struct Vfs {
    contents: SlotMap<FileId, Arc<RwLock<FileContent>>>,
    files: BiMap<FileId, VfsPath>,
}

impl Vfs {
    pub fn new() -> Self {
        Self {
            contents: SlotMap::with_key(),
            files: BiMap::new(),
        }
    }

    pub fn file_by_url(&self, url: &Uri) -> Option<FileId> {
        self.file_by_path(&url.to_vfs_path()?)
    }

    pub fn url_for_file(&self, file: FileId) -> Uri {
        Uri::from_vfs_path(self.path_by_file(file))
    }

    pub fn file_by_path(&self, path: &VfsPath) -> Option<FileId> {
        self.files.get_by_right(path).copied()
    }

    pub fn path_by_file(&self, file: FileId) -> &VfsPath {
        self.files.get_by_left(&file).unwrap()
    }

    pub fn content_by_file(&self, file: FileId) -> Arc<RwLock<FileContent>> {
        self.contents[file].clone()
    }

    pub fn set_path_content(&mut self, path: VfsPath, content: String) -> FileId {
        match self.file_by_path(&path) {
            Some(file) => {
                let mut contents = self.contents[file].write().unwrap();
                let len = contents.as_str().len();
                contents.replace(0..len, &content);
                file
            }
            None => {
                let file = self
                    .contents
                    .insert(Arc::new(FileContent::new(content).into()));
                self.files.insert(file, path);
                file
            }
        }
    }

    pub fn change_file_content(
        &mut self,
        file: FileId,
        new_text: &str,
        range: Option<Range<usize>>,
    ) {
        let mut content = self.contents[file].write().unwrap();
        let len = content.as_str().len();
        match range {
            Some(range) => {
                content.replace(range, new_text);
            }
            None => content.replace(0..len, new_text),
        }
    }

    pub fn remove_uri(&mut self, url: &Uri) -> Option<()> {
        let file = self.file_by_url(url)?;
        self.contents.remove(file);
        self.files.remove_by_left(&file);
        Some(())
    }
}

impl Default for Vfs {
    fn default() -> Self {
        Self::new()
    }
}
