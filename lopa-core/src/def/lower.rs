use itertools::Itertools;
use la_arena::{Arena, Idx};
use ustr::Ustr;

use crate::{
    def::ir::{self, Function},
    ide::{self},
    parsing::ast,
};

pub type FunctionId<'db> = Idx<Function<'db>>;

#[salsa::tracked]
pub struct IrFile<'db> {
    pub functions: Vec<ir::Function<'db>>,
    pub structs: Vec<ir::Struct<'db>>,
}

struct LowerCtx<'db> {
    db: &'db dyn salsa::Database,
    functions: Vec<ir::Function<'db>>,
    structs: Vec<ir::Struct<'db>>,
    root: ast::File,
    file: ide::File,
}

impl<'db> LowerCtx<'db> {
    pub fn new(db: &'db dyn salsa::Database, parse: ide::Parse<'db>, file: ide::File) -> Self {
        Self {
            functions: Default::default(),
            structs: Default::default(),
            root: parse.file(db),
            file,
            db,
        }
    }

    pub fn lower(mut self, file: ast::File) -> IrFile<'db> {
        for item in file.items() {
            self.item(item);
        }
        IrFile::new(self.db, self.functions, self.structs)
    }

    fn item(&mut self, item: ast::Item) {
        match item {
            ast::Item::FnItem(fn_item) => {
                if let Some(item) = self.fn_item(fn_item) {
                    self.functions.push(item);
                }
            }
            ast::Item::StructItem(struct_item) => {
                if let Some(item) = self.struct_item(struct_item) {
                    self.structs.push(item);
                }
            }
            ast::Item::ModItem(mod_item) => {}
        };
    }

    fn struct_item(&self, struct_item: ast::StructItem) -> Option<ir::Struct<'db>> {
        Some(ir::Struct::new(
            self.db,
            struct_item.name()?.text()?,
            ast::AstPtr::new(&struct_item),
            self.file,
        ))
    }

    fn fn_item(&self, fn_item: ast::FnItem) -> Option<ir::Function<'db>> {
        Some(ir::Function::new(
            self.db,
            fn_item.name()?.text()?,
            ast::AstPtr::new(&fn_item),
            self.file,
        ))
    }
}

/// Returns None if type was not found.
pub fn lower_type_expr<'db>(
    db: &'db dyn salsa::Database,
    ty: ast::TypeExpr,
) -> Option<ir::TypeExpr<'db>> {
    Some(match ty {
        ast::TypeExpr::PathType(path_type) => todo!(),
        ast::TypeExpr::NilableType(nilable_type) => {
            ir::TypeExpr::Nilable(Box::new(lower_type_expr(db, nilable_type.ty()?)?))
        }
        ast::TypeExpr::LitType(lit_type) => ir::TypeExpr::Lit(lit_type.kind()?),
        ast::TypeExpr::AnyType(_) => ir::TypeExpr::Any,
        ast::TypeExpr::UnitType(_) => ir::TypeExpr::Unit,
    })
}

pub fn lower_file<'db>(
    db: &'db dyn salsa::Database,
    parse: ide::Parse<'db>,
    file: ide::File,
) -> IrFile<'db> {
    let ctx = LowerCtx::new(db, parse, file);
    ctx.lower(parse.file(db))
}
