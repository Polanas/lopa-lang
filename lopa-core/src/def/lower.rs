use crate::{
    def::ir_def,
    ide::{self, Parse, diagnostics::Diagnostic},
    parsing::ast::{self, SyntaxNodePtr},
};
use rowan::ast::AstPtr;
use salsa::{Database, tracked};
use std::sync::Arc;

#[derive(salsa::Update, PartialEq, Eq, Clone, Debug)]
pub struct IrFile {
    pub ir: ir_def::File,
    pub diagnostics: Vec<Diagnostic>,
}

struct LowerContext {
    pub diagnostics: Vec<Diagnostic>,
}

impl LowerContext {
    fn new() -> Self {
        Self {
            diagnostics: Default::default(),
        }
    }
    fn lower_fn_item(&self, item: ast::FnItem) -> ir_def::FnItem {
        todo!()
    }

    fn lower_item(&self, item: ast::Item) -> ir_def::Item {
        match item {
            ast::Item::FnItem(fn_item) => ir_def::Item::FnItem(self.lower_fn_item(fn_item)),
        }
    }

    fn lower_file(&self, file: ast::File) -> ir_def::File {
        ir_def::File {
            node_ptr: SyntaxNodePtr::new(&file.0),
            items: file.items().map(|i| self.lower_item(i)).collect::<_>(),
        }
    }
}

pub fn lower_file(parse: Arc<Parse>) -> IrFile {
    let ctx = LowerContext::new();

    IrFile {
        ir: ctx.lower_file(parse.file()),
        diagnostics: ctx.diagnostics,
    }
}
