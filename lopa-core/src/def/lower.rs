use itertools::Itertools;
use la_arena::{Arena, Idx};
use rowan::ast::AstNode;

use crate::{
    def::{
        ir::{self, BareFn, Function, Type},
        scope,
    },
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
            ast::Item::ImplItem(impl_item) => {},
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

pub fn lower_type_expr<'db>(
    db: &'db dyn salsa::Database,
    file: ide::File,
    ty: ast::TypeExpr,
) -> ir::Type<'db> {
    match ty {
        ast::TypeExpr::PathType(path_type) => {
            let Some(path) = path_type.value() else {
                return ir::Type::Unknown(path_type.syntax().text().to_string().into());
            };
            let module_scope = scope::module_scope(db, file);
            let Some((_, def)) = module_scope
                .types()
                .find(|t| t.0.0 == path.segments().last().unwrap())
            else {
                return ir::Type::Unknown(path_type.syntax().text().to_string().into());
            };

            match def {
                ir::ModuleDef::Function(_) => unreachable!(),
                ir::ModuleDef::Struct(strct) => ir::Type::Struct(*strct),
            }
        }
        ast::TypeExpr::NilableType(nilable_type) => {
            let Some(ty) = nilable_type.ty() else {
                return ir::Type::Unknown(nilable_type.syntax().text().to_string().into());
            };
            ir::Type::Nilable(Box::new(lower_type_expr(db, file, ty)))
        }
        ast::TypeExpr::LitType(lit_type) => {
            let Some(kind) = lit_type.kind() else {
                return ir::Type::Unknown(lit_type.syntax().text().to_string().into());
            };

            ir::Type::Lit(kind)
        }
        ast::TypeExpr::AnyType(_) => ir::Type::Any,
        ast::TypeExpr::UnitType(_) => ir::Type::Unit,
        ast::TypeExpr::FnType(fn_type) => ir::Type::BareFn(BareFn {
            params: fn_type
                .param_list()
                .map(|list| {
                    list.params()
                        .filter_map(|param| {
                            param
                                .ty()
                                .map(|ty| lower_type_expr(db, file, ty))
                                .map(|ty| (ty, param.name()))
                        })
                        .map(|(ty, n)| ir::Param {
                            name: n.and_then(|n| n.text()),
                            ty,
                        })
                        .collect_vec()
                })
                .unwrap_or_default(),
            output: fn_type
                .output()
                .and_then(|o| o.ty())
                .map(|ty| lower_type_expr(db, file, ty))
                .unwrap_or_else(|| Type::Unit)
                .into(),
        }),
    }
}

pub fn lower_file<'db>(
    db: &'db dyn salsa::Database,
    parse: ide::Parse<'db>,
    file: ide::File,
) -> IrFile<'db> {
    let ctx = LowerCtx::new(db, parse, file);
    ctx.lower(parse.file(db))
}
