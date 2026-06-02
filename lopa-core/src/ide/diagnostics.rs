use crate::{
    def::{
        self,
        ir::{Struct, Type},
        scope,
    },
    ide::{self, File, Files, base::FileRange, diagnostics, impls, parse},
    parsing::parser::{ErrorKind as SyntaxErrorKind, ParseError},
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
    pub range: TextRange,
    pub kind: DiagnosticKind,
    pub notes: Vec<(FileRange, String)>,
}

impl Diagnostic {
    pub fn new(range: TextRange, kind: DiagnosticKind) -> Self {
        Self {
            range,
            kind,
            notes: Default::default(),
        }
    }

    pub fn severity(&self) -> Severity {
        match &self.kind {
            DiagnosticKind::SyntaxError(_) => Severity::Error,
            DiagnosticKind::TypeError(_) => Severity::Error,
        }
    }

    pub fn message(self) -> String {
        match self.kind {
            DiagnosticKind::SyntaxError(kind) => kind.to_string(),
            DiagnosticKind::TypeError(err) => err.message,
        }
    }

    pub fn code(&self) -> &'static str {
        match &self.kind {
            DiagnosticKind::SyntaxError(_) => "syntax_error",
            DiagnosticKind::TypeError(_) => "type_error",
        }
    }
}

impl From<ParseError> for Diagnostic {
    fn from(value: ParseError) -> Self {
        Self::new(value.range, DiagnosticKind::SyntaxError(value.kind))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiagnosticKind {
    SyntaxError(SyntaxErrorKind),
    TypeError(infer::TypeErrorKind),
}

pub fn diagnostics(db: &dyn salsa::Database, file: File, files: Files) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];

    let parse = parse(db, file);
    diagnostics.extend(parse.errors(db).clone().into_iter().map(Diagnostic::from));

    let ir = ide::lower_structs_fns(db, file);
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
                .filter_map(|(err, r)| r.map(|r| (err, r)))
                .map(|(err, range)| Diagnostic {
                    range,
                    kind: DiagnosticKind::TypeError(infer::TypeErrorKind { message: err }),
                    notes: vec![],
                }),
        );
    }

    diagnostics
}
