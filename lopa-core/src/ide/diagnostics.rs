use std::ops::Range;

use crate::def::{
    UseTreeId,
    hir::{EnumId, FunctionId, ModuleId, StructId, UseItemId},
};

//TODO: refactor this (add more kinds for each class of error like in rust)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(clippy::enum_variant_names)]
pub enum DiagnosticKind {
    SyntaxError,
    TypeError,
    ModuleError,
}

#[derive(Clone, PartialEq, Debug, Eq, Hash)]
pub enum DiagnosticLocation {
    Module(ModuleId),
    Struct(StructId),
    Enum(EnumId),
    Function(FunctionId),
    UseTree {
        use_id: UseItemId,
        tree_id: UseTreeId,
    },
    Param {
        fn_item: FunctionId,
        param_num: usize,
    },
    Range(Range<usize>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[salsa::accumulator]
pub struct Diagnostic {
    pub message: String,
    pub location: DiagnosticLocation,
    pub kind: DiagnosticKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderedDiagnostic {
    pub message: String,
    pub range: Range<usize>,
    pub kind: DiagnosticKind,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl RenderedDiagnostic {
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
