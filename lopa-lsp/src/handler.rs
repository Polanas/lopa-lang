use lopa_core::ide::TextRange;
use tower_lsp_server::ls_types::{Diagnostic, TextEdit, Uri};

use crate::{State, convert, vfs_ext::VfsExt};

pub fn format(state: State, uri: &Uri) -> TextEdit {
    let vfs = state.vfs.read().unwrap();
    let file = vfs.file_by_uri(uri).unwrap();
    let analysis = state.analysis.lock().unwrap();
    let formatted = analysis.format(file);
    let range = {
        let contents = vfs.content_by_file(file);
        let mut contents = contents.write().unwrap();
        let len = contents.len() as u32;
        convert::to_range(&mut contents, TextRange::new(0.into(), len.into()))
    };

    TextEdit::new(range, formatted)
}

pub fn diagnostics(state: State, uri: &Uri) -> Vec<Diagnostic> {
    let (file, contents) = {
        let vfs = state.vfs.read().unwrap();
        let file = vfs.file_by_uri(uri).expect("TODO: handle");
        (file, vfs.content_by_file(file))
    };

    let analysis = state.analysis.lock().unwrap();
    let diagnostics = analysis.diagnostics(file);
    convert::to_diagnostics(contents, diagnostics)
}
