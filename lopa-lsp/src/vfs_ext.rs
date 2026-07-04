use lopa_core::{ide::File, vfs::Vfs};
use tower_lsp_server::ls_types::Uri;

pub trait VfsExt {
    fn file_by_uri(&self, url: &Uri) -> Option<File>;
    fn url_for_file(&self, file: File) -> Uri;
    fn remove_uri(&mut self, url: &Uri) -> Option<()>;
}

impl VfsExt for Vfs {
    fn file_by_uri(&self, url: &Uri) -> Option<File> {
        self.file_by_path(&url.to_file_path().unwrap().to_path_buf())
    }

    fn url_for_file(&self, file: File) -> Uri {
        Uri::from_file_path(self.path_by_file(file)).unwrap()
    }

    fn remove_uri(&mut self, url: &Uri) -> Option<()> {
        let file = self.file_by_uri(url)?;
        self.contents.remove(&file);
        self.files.remove_by_left(&file);
        Some(())
    }
}
