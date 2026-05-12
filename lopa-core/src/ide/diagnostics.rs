use std::sync::{Arc, RwLock};

use crate::{
    ide::{File, base::FileRange, lower_file, parse},
    parsing::parser::{ErrorKind as SyntaxErrorKind, ParseError},
};
use itertools::Itertools;
use rowan::{TextRange, TextSize};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
        }
    }

    pub fn message(&self) -> String {
        match &self.kind {
            DiagnosticKind::SyntaxError(kind) => kind.to_string(),
        }
    }

    pub fn code(&self) -> &'static str {
        match &self.kind {
            DiagnosticKind::SyntaxError(_) => "syntax_error",
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
}

pub fn diagnostics(db: &dyn salsa::Database, file: File) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];

    let parse = parse(db, file);
    diagnostics.extend(parse.errors(db).clone().into_iter().map(Diagnostic::from));

    let ir = lower_file(db, file);
    // notify_rust::Notification::new()
    //     .summary("functions")
    //     .body(&ir.functions(db).iter().map(|f| f.name(db)).join(" "))
    //     .show()
    //     .unwrap();

    diagnostics
}
