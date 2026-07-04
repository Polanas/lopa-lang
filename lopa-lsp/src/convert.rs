use std::ops::Range;

use lopa_core::{
    ide::{self, Diagnostic, File, RenderedDiagnostic, Severity},
    vfs::{FileContent, Vfs},
};
use tower_lsp_server::ls_types::{
    self, DiagnosticSeverity, NumberOrString, Position, Range as LspRange,
};

pub fn from_range(vfs: &Vfs, file: File, range: LspRange) -> Range<usize> {
    let content = vfs.content_by_file(file);

    let start_offset = content.pos_by_line_col(range.start.line, range.start.character);
    let end_offset = content.pos_by_line_col(range.end.line, range.end.character);
    (start_offset as usize)..(end_offset as usize)
}

pub fn to_range(content: &FileContent, range: Range<usize>) -> LspRange {
    let (line1, col1) = content.line_col_by_pos(range.start as _);
    let (line2, col2) = content.line_col_by_pos(range.end as _);

    LspRange::new(Position::new(line1, col1), Position::new(line2, col2))
}

pub fn to_lsp_diagnostics(
    content: &FileContent,
    diagnostics: Vec<RenderedDiagnostic>,
) -> Vec<ls_types::Diagnostic> {
    let mut res = Vec::with_capacity(diagnostics.len() * 2);
    for diagnostic in diagnostics {
        let primary_diagnostic = ls_types::Diagnostic {
            range: to_range(&content, diagnostic.range.clone()),
            severity: Some(match diagnostic.severity() {
                Severity::Error => DiagnosticSeverity::ERROR,
                Severity::Warning => DiagnosticSeverity::WARNING,
                Severity::Info => DiagnosticSeverity::INFORMATION,
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
