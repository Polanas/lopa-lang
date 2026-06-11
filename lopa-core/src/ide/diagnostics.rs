use crate::{
    def::{
        self, ir,
        lower::{self, impl_blocks},
        scope,
    },
    ide::{self, File, base::FileRange, diagnostics, impl_map, parse},
    ty::infer,
};
use notify_rust::Notification;
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

//TODO: figure out why this doesnt work and accumulated diagostics arent returned (it runs every time?)
#[salsa::tracked]
pub fn file_diagnostics(db: &dyn salsa::Database, file: File) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];

    let parse = parse(db, file);
    diagnostics.extend(
        parse
            .errors(db)
            .clone()
            .into_iter()
            .map(|e| Diagnostic::new(e.range, DiagnosticKind::SyntaxError, e.kind.to_string())),
    );

    diagnostics.extend(scope::resolve_imports(db, file));
    diagnostics.extend(
        //avoiding accumulators as they don't work with cycle_result (in `resolve_path`)
        ide::impl_map::accumulated::<Diagnostic>(db, file.source_root(db))
            .into_iter()
            .cloned(),
    );
    diagnostics.extend(
        scope::module_scope_with_source_map::accumulated::<Diagnostic>(db, file)
            .into_iter()
            .cloned(),
    );
    let ir = lower::module_items(db, file);
    let module_scope = scope::module_scope(db, file);
    //avoiding accumulators as they don't work with cycle_result (in `resolve_path`)
    diagnostics.extend(module_scope.diagnostics().to_vec());
    for enum_item in ir.enums(db) {
        diagnostics.extend(
            def::ir::enum_fields::accumulated::<Diagnostic>(db, *enum_item)
                .into_iter()
                .cloned(),
        );
    }
    for struct_item in ir.structs(db) {
        diagnostics.extend(
            def::ir::struct_fields::accumulated::<Diagnostic>(db, *struct_item)
                .into_iter()
                .cloned(),
        );
    }
    for enum_item in ir.enums(db) {
        diagnostics.extend(
            def::ir::enum_fields::accumulated::<Diagnostic>(db, *enum_item)
                .into_iter()
                .cloned(),
        );
    }
    for func in ir.functions(db) {
        diagnostics.extend(
            infer::infer_function::accumulated::<Diagnostic>(db, *func)
                .into_iter()
                .cloned(),
        );
    }
    for impl_block in lower::impl_blocks(db, file).impl_blocks(db).iter() {
        for func in impl_block.functions(db) {
            diagnostics.extend(
                infer::infer_function::accumulated::<Diagnostic>(db, *func)
                    .into_iter()
                    .cloned(),
            );
        }
    }

    diagnostics
}
