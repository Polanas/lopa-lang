use itertools::Itertools;
use ustr::Ustr;

use crate::{
    def::ir::{self, Function},
    ide::{self, diagnostics::Diagnostic},
    parsing::{ast, parser},
};

#[derive(Debug, Clone, PartialEq, Eq, salsa::Update)]
pub struct FunctionName(pub Ustr);

impl identity_hash::IdentityHashable for FunctionName {}

impl std::hash::Hash for FunctionName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.precomputed_hash());
    }
}

indexmap_hash! {
    FunctionMap<'db>(indexmap::IndexMap<
        FunctionName,
        ir::Function<'db>,
        identity_hash::BuildIdentityHasher<FunctionName>>)
}

#[salsa::tracked]
pub struct IrFile<'db> {
    pub functions: Vec<ir::Function<'db>>,
    pub diagnostics: Vec<Diagnostic>,
}

struct LowerCtx<'db> {
    db: &'db dyn salsa::Database,
    diagnostics: Vec<Diagnostic>,
    functions: Vec<ir::Function<'db>>,
    ast_file: ast::File,
    file: ide::File,
}

impl<'db> LowerCtx<'db> {
    pub fn new(db: &'db dyn salsa::Database, parse: ide::Parse<'db>, file: ide::File) -> Self {
        Self {
            diagnostics: Default::default(),
            functions: Default::default(),
            ast_file: parse.file(db),
            file,
            db,
        }
    }

    pub fn lower(mut self, file: ast::File) -> IrFile<'db> {
        for item in file.items() {
            self.item(item);
        }
        IrFile::new(
            self.db,
            self.functions,
            // self.functions
            //     .into_iter()
            //     .map(|f| (FunctionName(f.name(self.db)), f))
            //     .collect::<indexmap::IndexMap<_, _, identity_hash::BuildIdentityHasher<FunctionName>>>()
            //     .into(),
            self.diagnostics,
        )
    }

    fn item(&mut self, item: ast::Item) {
        match item {
            ast::Item::FnItem(fn_item) => {
                if let Some(item) = self.fn_item(fn_item) {
                    self.functions.push(item);
                }
            }
            ast::Item::ModItem(mod_item) => {}
        };
    }

    fn fn_item(&self, fn_item: ast::FnItem) -> Option<ir::Function<'db>> {
        Some(ir::Function::new(
            self.db,
            fn_item.name()?.text()?,
            fn_item
                .params()?
                .params()
                .filter_map(|p| self.param(p))
                .collect_vec(),
            fn_item
                .output()
                .and_then(|o| o.ty())
                .and_then(|o| self.type_expr(o)),
            ast::AstPtr::new(&fn_item),
            self.file,
        ))
    }

    fn param(&self, param: ast::FnParam) -> Option<ir::FnParam<'db>> {
        Some(ir::FnParam::new(
            self.db,
            // param.pattern()?.text()?,
            lower_type_expr(param.ty()?)?,
        ))
    }
}

pub fn lower_type_expr(item: ast::TypeExpr) -> Option<ir::TypeExpr> {
    Some(match item {
        ast::TypeExpr::PathType(path_ty) => ir::TypeExpr::PathType(path_type(path_ty)?),
        ast::TypeExpr::NilableType(nilable_ty) => {
            ir::TypeExpr::NilableType(nilable_type(nilable_ty)?)
        }
        ast::TypeExpr::LitType(lit_ty) => ir::TypeExpr::LitType(lit_type(lit_ty)?),
        ast::TypeExpr::AnyType(any_ty) => ir::TypeExpr::AnyType(any_type(any_ty)?),
    })
}

fn path_type(item: ast::PathType) -> Option<ir::PathType> {
    Some(ir::PathType {
        value: path(item.value()?)?,
    })
}

fn path(item: ast::Path) -> Option<ir::Path> {
    Some(ir::Path {
        segments: item.segments().collect_vec(),
    })
}

fn lit_type(item: ast::LitType) -> Option<ir::LitType> {
    Some(ir::LitType { kind: item.kind()? })
}

fn any_type(item: ast::AnyType) -> Option<ir::AnyType> {
    Some(ir::AnyType {})
}

fn nilable_type(item: ast::NilableType) -> Option<ir::NilableType> {
    Some(ir::NilableType {
        value: lower_type_expr(item.ty()?)?.into(),
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

#[cfg(test)]
mod test {
    use std::sync::Arc;

    // use super::*;
    // use crate::{
    //     def::lower::{IrFile, lower_file},
    //     parsing::parser::{self, Parse},
    // };

    #[test]
    fn func() {
        // let parse: Arc<Parse> = parser::parse("fn test() {print(\"hello world!\");}").into();
        // assert!(parse.errors.is_empty());
        // insta::assert_debug_snapshot!(lower_file(parse).ir)
    }
}
