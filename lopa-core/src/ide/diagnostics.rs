use std::{ops::Range, sync::Arc};

use crate::{common::PathNodeType, def::{
    BodyMap, HirNodeId, TypeExprId, UseTreeId, hir::{ClassId, EnumId, FunctionId, ModuleId, StructId, TraitId, UseItemId},
}};

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
    Class(ClassId),
    Trait(TraitId),
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
    HirNode {
        id: HirNodeId,
        source: Arc<BodyMap>,
    },
    PathSegment {
        id: HirNodeId,
        source: Arc<BodyMap>,
        offset: usize,
        node_type: PathNodeType,
    },
    Range(Range<usize>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, salsa::SalsaValue)]
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
