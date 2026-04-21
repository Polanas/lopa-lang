use tower_lsp_server::ls_types::{Diagnostic, Uri};

use crate::{State, convert};

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
