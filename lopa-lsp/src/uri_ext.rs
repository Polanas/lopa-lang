use lopa_core::ide::base::VfsPath;
use tower_lsp_server::ls_types::Uri;

pub trait UrlExt {
    fn to_vfs_path(&self) -> Option<VfsPath>;
    fn from_vfs_path(path: &VfsPath) -> Self;
}

impl UrlExt for Uri {
    fn to_vfs_path(&self) -> Option<VfsPath> {
        if self.scheme().as_str() == "file"
            && let Some(path) = self.to_file_path()
        {
            Some(VfsPath(path.to_string_lossy().to_string()))
        } else {
            None
        }
    }

    fn from_vfs_path(path: &VfsPath) -> Self {
        Uri::from_file_path(&path.0).unwrap()
    }
}
