use std::{ops::Range, path::PathBuf, sync::Arc};

use bimap::BiMap;
use crop::Rope;
use slotmap::SlotMap;
use tower_lsp_server::ls_types::Uri;

use crate::uri_ext::UrlExt as _;

slotmap::new_key_type! { pub struct FileId; }

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct VfsPath(pub String);

//Maps between filesystem paths and `FileId`s, store file contents
pub struct Vfs {
    contents: SlotMap<FileId, Rope>,
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

    pub fn content_by_file(&self, file: FileId) -> Rope {
        self.contents[file].clone()
    }

    pub fn set_path_content(&mut self, path: VfsPath, content: String) -> FileId {
        match self.file_by_path(&path) {
            Some(file) => {
                let rope = &mut self.contents[file];
                rope.replace(0..rope.byte_len(), &content);
                file
            }
            None => {
                let file = self.contents.insert(Rope::from(content));
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
        let rope = &mut self.contents[file];
        match range {
            Some(range) => {
                if range.start == range.end {
                    rope.insert(range.start, new_text);
                } else {
                    rope.delete(range);
                }
            }
            None => rope.replace(0..rope.byte_len(), new_text),
        }
    }

    pub fn remove_uri(&mut self, url: &Uri) -> Option<()> {
        let file = self.file_by_url(url)?;
        self.contents.remove(file);
        self.files.remove_by_left(&file);
        Some(())
    }
}
