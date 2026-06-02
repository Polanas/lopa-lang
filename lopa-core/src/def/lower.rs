use itertools::Itertools;
use la_arena::{Arena, Idx};
use rowan::ast::AstNode;
use ustr::Ustr;

use crate::{
    def::{
        ir::{self, BareFn, Function, Type},
        scope,
    },
    ide::{self},
    parsing::ast,
};

#[salsa::tracked]
pub struct MoudleIr<'db> {
    #[returns(ref)]
    pub structs_fns_ir: StructsFnsIr<'db>,
    #[returns(ref)]
    pub impl_blocks_ir: ImplBlocksIr<'db>,
}

#[salsa::tracked(debug)]
pub struct StructsFnsIr<'db> {
    #[returns(ref)]
    pub functions: Vec<ir::Function<'db>>,
    #[returns(ref)]
    pub structs: Vec<ir::Struct<'db>>,
}

#[salsa::tracked(debug)]
pub struct ImplBlocksIr<'db> {
    #[returns(ref)]
    pub impl_blocks: Vec<ImplBlock<'db>>,
}

#[salsa::tracked(debug)]
pub struct ImplBlock<'db> {
    pub implementee: Type<'db>,
    pub impl_ty: Option<Type<'db>>,
    #[returns(ref)]
    pub methods: Vec<ir::ImplFunction<'db>>,
}

struct LowerCtx<'db> {
    db: &'db dyn salsa::Database,
    functions: Vec<ir::Function<'db>>,
    structs: Vec<ir::Struct<'db>>,
    impl_blocks: Vec<ImplBlock<'db>>,
    file: ide::File,
}

impl<'db> LowerCtx<'db> {
    pub fn new(db: &'db dyn salsa::Database, file: ide::File) -> Self {
        Self {
            functions: Default::default(),
            structs: Default::default(),
            impl_blocks: Default::default(),
            file,
            db,
        }
    }

    pub fn lower_impls(mut self, file: ast::File) -> ImplBlocksIr<'db> {
        for item in file.items().filter_map(|i| match i {
            ast::Item::ImplItem(i) => Some(i),
            _ => None,
        }) {
            if let Some(item) = self.impl_item(item) {
                self.impl_blocks.push(item);
            }
        }
        ImplBlocksIr::new(self.db, self.impl_blocks)
    }

    pub fn lower_structs_fns(mut self, file: ast::File) -> StructsFnsIr<'db> {
        for item in file.items() {
            self.type_item(item);
        }

        StructsFnsIr::new(self.db, self.functions, self.structs)
    }

    fn impl_item(&mut self, item: ast::ImplItem) -> Option<ImplBlock<'db>> {
        let implementee = item
            .implementee()
            .map(|i| lower_type_expr(self.db, self.file, i))?;
        let impl_ty = item
            .impl_ty()
            .and_then(|t| t.ty())
            .map(|i| lower_type_expr(self.db, self.file, i));
        let methods = item
            .functions()
            .filter_map(|f| self.fn_item(f))
            .map(|f| ir::ImplFunction::new(self.db, f, implementee.clone()))
            .collect_vec();
        Some(ImplBlock::new(self.db, implementee, impl_ty, methods))
    }

    fn type_item(&mut self, item: ast::Item) {
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
            _ => {}
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
    lower_type_expr_with_self(db, file, ty, None)
}

pub fn lower_type_expr_with_self<'db>(
    db: &'db dyn salsa::Database,
    file: ide::File,
    ty: ast::TypeExpr,
    owner: Option<Type<'db>>,
) -> ir::Type<'db> {
    match ty {
        ast::TypeExpr::PathType(path_type) => resolve_type_path(db, file, path_type),
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
        ast::TypeExpr::SelfType(_) => owner.unwrap_or_else(|| Type::Unknown(Ustr::from("Self"))),
        ast::TypeExpr::DynType(dyn_type) => {
            let Some(path) = dyn_type.path() else {
                return ir::Type::Unknown("".into());
            };
            resolve_type_path(db, file, ast::PathType(path.syntax().clone()))
        }
    }
}

fn resolve_type_path<'db>(
    db: &'db dyn salsa::Database,
    file: ide::File,
    path_type: ast::PathType,
) -> Type<'_> {
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

#[salsa::tracked]
pub fn lower_structs_fns<'db>(db: &'db dyn salsa::Database, file: ide::File) -> StructsFnsIr<'db> {
    let parse = ide::parse(db, file);
    let ctx = LowerCtx::new(db, file);
    ctx.lower_structs_fns(parse.file(db))
}

#[salsa::tracked]
pub fn lower_impl_blocks<'db>(db: &'db dyn salsa::Database, file: ide::File) -> ImplBlocksIr<'db> {
    let parse = ide::parse(db, file);
    let ctx = LowerCtx::new(db, file);
    ctx.lower_impls(parse.file(db))
}
#[salsa::tracked(returns(ref))]
pub fn lower_module<'db>(db: &'db dyn salsa::Database, file: ide::File) -> MoudleIr<'db> {
    MoudleIr::new(db, lower_structs_fns(db, file), lower_impl_blocks(db, file))
}
