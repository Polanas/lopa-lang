use std::sync::{Arc, RwLock};

use lopa_core::ide::{File, FileContent, TextRange, diagnostics::Diagnostic};
use tower_lsp_server::ls_types::{self, DiagnosticSeverity, NumberOrString, Position, Range, Uri};

use lopa_core::vfs::Vfs;

pub fn from_range(vfs: &Vfs, file: File, range: Range) -> TextRange {
    let content = vfs.content_by_file(file);
    let mut content = content.write().unwrap();

    let start_offset = content.pos_by_line_col(range.start.line, range.start.character);
    let end_offset = content.pos_by_line_col(range.end.line, range.end.character);
    TextRange::new(start_offset.into(), end_offset.into())
}

pub fn to_range(content: &mut FileContent, range: TextRange) -> Range {
    let (line1, col1) = content.line_col_by_pos(range.start().into());
    let (line2, col2) = content.line_col_by_pos(range.end().into());

    Range::new(Position::new(line1, col1), Position::new(line2, col2))
}

pub fn to_diagnostics(
    content: Arc<RwLock<FileContent>>,
    diagnostics: Vec<Diagnostic>,
) -> Vec<ls_types::Diagnostic> {
    let mut res = Vec::with_capacity(diagnostics.len() * 2);
    let mut content = content.write().unwrap();
    for diagnostic in diagnostics {
        let primary_diagnostic = ls_types::Diagnostic {
            range: to_range(&mut content, diagnostic.range),
            severity: Some(match diagnostic.severity() {
                lopa_core::ide::diagnostics::Severity::Error => DiagnosticSeverity::ERROR,
                lopa_core::ide::diagnostics::Severity::Warning => DiagnosticSeverity::WARNING,
                lopa_core::ide::diagnostics::Severity::Info => DiagnosticSeverity::INFORMATION,
            }),
            code: Some(NumberOrString::String(diagnostic.code().into())),
            code_description: None,
            message: diagnostic.message,
            source: None,
            related_information: None,
            tags: None,
            data: None,
        };

        // for (file_range, message) in diagnostic.notes {
        //     if file_range.file_id != file {
        //         continue;
        //     }
        //
        //     res.push(ls_types::Diagnostic {
        //         range: todo!(),
        //         severity: Some(DiagnosticSeverity::HINT),
        //         code: todo!(),
        //         code_description: todo!(),
        //         source: todo!(),
        //         related_information: todo!(),
        //         message,
        //         tags: None,
        //         data: None,
        //     })
        // }

        res.push(primary_diagnostic);
    }
    res
}
