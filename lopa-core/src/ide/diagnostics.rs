use crate::{
    def::{
        self,
        ir::{Struct, Type},
        lower, scope,
    },
    ide::{self, File, base::FileRange, diagnostics, impls, parse},
    parsing::{
        self,
        parser::{ParseError, SyntaxErrorKind},
    },
    ty::infer,
};
use rowan::{TextRange, TextSize};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[salsa::accumulator]
pub struct Diagnostic {
    pub message: String,
    pub range: TextRange,
    pub kind: DiagnosticKind,
    pub notes: Vec<(FileRange, String)>,
}

impl Diagnostic {
    pub fn new(range: TextRange, kind: DiagnosticKind, message: String) -> Self {
        Self {
            range,
            kind,
            message,
            notes: Default::default(),
        }
    }

    pub fn severity(&self) -> Severity {
        match &self.kind {
            DiagnosticKind::SyntaxError => Severity::Error,
            DiagnosticKind::TypeError => Severity::Error,
            DiagnosticKind::ModuleError => Severity::Error,
        }
    }

    pub fn code(&self) -> &'static str {
        match &self.kind {
            DiagnosticKind::SyntaxError => "syntax_error",
            DiagnosticKind::TypeError => "type_error",
            DiagnosticKind::ModuleError => "module_error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiagnosticKind {
    SyntaxError,
    TypeError,
    ModuleError,
}

pub fn diagnostics(db: &dyn salsa::Database, file: File) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];

    let parse = parse(db, file);
    diagnostics.extend(
        parse
            .errors(db)
            .clone()
            .into_iter()
            .map(|e| Diagnostic::new(e.range, DiagnosticKind::SyntaxError, e.kind.to_string())),
    );

    let ir = lower::module_items(db, file);
    diagnostics.extend(
        lower::module_items::accumulated::<Diagnostic>(db, file)
            .into_iter()
            .cloned(),
    );
    for struct_item in ir.structs(db) {
        diagnostics.extend(
            def::ir::struct_fields::accumulated::<Diagnostic>(db, *struct_item)
                .into_iter()
                .cloned(),
        );
    }
    for func in ir.functions(db) {
        diagnostics.extend(
            infer::type_diagnostics(db, *func)
                .into_iter()
                .filter_map(|(message, r)| r.map(|range| (message, range)))
                .map(|(message, range)| Diagnostic::new(range, DiagnosticKind::TypeError, message)),
        );
    }

    diagnostics
}
