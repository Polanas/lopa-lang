use std::{collections::HashMap, sync::Arc};

use itertools::Itertools as _;
use la_arena::{Arena, Idx};

use crate::{
    common::Symbol,
    def::{AstIdMap, ast_id_map, hir::*},
    ide::{self, Root},
    parsing::{self, AstNode as _},
};

// struct LowerBodyCtx<'db, 'ast, 's> {
//     db: &'db dyn salsa::Database,
//     expr: parsing::Expr<'ast>,
//     source: &'s str,
//     ctx: BodyCtx,
// }

// impl<'db, 'ast, 's> LowerBodyCtx<'db, 'ast, 's> {
//     fn new(db: &'db dyn salsa::Database, expr: parsing::Expr<'ast>, source: &'s str) -> Self {
//         Self {
//             db,
//             expr,
//             ctx: BodyCtx::default(),
//             source,
//         }
//     }
//
//     fn alloc_expr(&mut self, expr: Expr) -> ExprId {
//         self.ctx.exprs.alloc(expr)
//     }
//
//     fn alloc_pat(&mut self, pat: Pat) -> PatId {
//         self.ctx.pats.alloc(pat)
//     }
//
//     fn alloc_type_expr(&mut self, type_expr: TypeExpr) -> TypeExprId {
//         self.ctx.type_exprs.alloc(type_expr)
//     }
//
//     fn missing_type_expr(&mut self) -> TypeExprId {
//         self.ctx.type_exprs.alloc(TypeExpr::Missing)
//     }
//
//     fn missing_expr(&mut self) -> ExprId {
//         self.alloc_expr(Expr::Missing)
//     }
//
//     fn missing_pat(&mut self) -> PatId {
//         self.alloc_pat(Pat::Missing)
//     }
//
//     fn alloc_stmt(&mut self, stmt: Stmt) -> StmtId {
//         self.ctx.stmts.alloc(stmt)
//     }
//
//     fn lower(mut self) -> Body<'db> {
//         let expr = self.expr(self.expr);
//         Body::new(
//             self.db,
//             self.ctx.exprs,
//             self.ctx.pats,
//             self.ctx.type_exprs,
//             self.ctx.stmts,
//             expr,
//         )
//     }
//
//     fn stmt(&mut self, stmt: parsing::Stmt<'ast>) -> Option<StmtId> {
//         Some(match stmt {
//             parsing::Stmt::LetStmt(let_stmt) => {
//                 let (pat, expr) = (let_stmt.pattern()?, let_stmt.expr()?);
//                 let pat = self.pat(pat);
//                 let expr = self.expr(expr);
//                 let ty = let_stmt.ty().map(|ty| self.type_expr(ty));
//                 self.alloc_stmt(Stmt::Let { pat, ty, expr })
//             }
//             parsing::Stmt::ExprStmt(expr_stmt) => {
//                 let expr = self.expr(expr_stmt.expr()?);
//                 self.alloc_stmt(Stmt::Expr {
//                     expr,
//                     semi: expr_stmt.semi_token().map(|_| ()),
//                 })
//             }
//         })
//     }
//
//     fn pat(&mut self, pat: parsing::Pattern) -> PatId {
//         match pat {
//             parsing::Pattern::NamePattern(name_pattern) => {
//                 let pat = name_pattern
//                     .name()
//                     .and_then(|n| n.text(self.source))
//                     .map(|name| Symbol::new(self.db, name))
//                     .map(Pat::Name)
//                     .unwrap_or_else(|| Pat::Missing);
//                 self.alloc_pat(pat)
//             }
//             parsing::Pattern::PathPattern(path_pattern) => {
//                 let path = path_pattern.path().map(|p| self.path(p));
//                 self.alloc_pat(path.map(Pat::Path).unwrap_or_else(|| Pat::Missing))
//             }
//             parsing::Pattern::WildcardPattern(_) => self.alloc_pat(Pat::Wildcard),
//         }
//     }
//
//     fn path(&mut self, path: parsing::Path) -> Path {
//         Path {
//             segments: path
//                 .segments()
//                 .filter_map(|s| self.path_segment(s))
//                 .collect_vec(),
//         }
//     }
//
//     fn path_segment(&mut self, path_segment: parsing::PathSegment) -> Option<PathSegment> {
//         Some(PathSegment {
//             ident: Symbol::new(self.db, path_segment.ident(self.source)?),
//             generic_args: path_segment
//                 .generic_args()
//                 .map(|args| args.types())
//                 .into_iter()
//                 .flatten()
//                 .map(|ty| self.type_expr(ty))
//                 .collect_vec(),
//         })
//     }
//
//     fn pat_opt(&mut self, pat: Option<parsing::Pattern>) -> PatId {
//         pat.map(|e| self.pat(e))
//             .unwrap_or_else(|| self.missing_pat())
//     }
//
//     fn type_expr(&mut self, type_expr: parsing::TypeExpr) -> TypeExprId {
//         fn inner(this: &mut LowerBodyCtx, type_expr: parsing::TypeExpr) -> Option<TypeExprId> {
//             Some(match type_expr {
//                 parsing::TypeExpr::DynType(dyn_type) => {
//                     let path = this.path(dyn_type.path()?);
//                     this.alloc_type_expr(TypeExpr::Dyn(path))
//                 }
//                 parsing::TypeExpr::ParenType(paren_type) => {
//                     let ty = paren_type
//                         .type_expr()
//                         .map(|ty| this.type_expr(ty))
//                         .unwrap_or_else(|| this.missing_type_expr());
//                     this.alloc_type_expr(TypeExpr::Paren(ty))
//                 }
//                 parsing::TypeExpr::PathType(path_type) => {
//                     let path = this.path(path_type.value()?);
//                     this.alloc_type_expr(TypeExpr::Path(path))
//                 }
//                 parsing::TypeExpr::NilableType(nilable_type) => {
//                     let ty = nilable_type
//                         .ty()
//                         .map(|nilable| this.type_expr(nilable))
//                         .unwrap_or_else(|| this.missing_type_expr());
//                     this.alloc_type_expr(TypeExpr::Nilable(ty))
//                 }
//                 parsing::TypeExpr::LitType(lit_type) => {
//                     this.alloc_type_expr(TypeExpr::Lit(lit_type.kind()?))
//                 }
//                 parsing::TypeExpr::FnType(fn_type) => {
//                     let output = fn_type.output().map(|o| {
//                         o.ty()
//                             .map(|ty| this.type_expr(ty))
//                             .unwrap_or_else(|| this.missing_type_expr())
//                     });
//                     let params = fn_type
//                         .param_list()
//                         .map(|p| p.params())
//                         .into_iter()
//                         .flatten()
//                         .filter_map(|p| this.fn_type_param(p))
//                         .collect_vec();
//                     this.alloc_type_expr(TypeExpr::Fn { params, output })
//                 }
//                 parsing::TypeExpr::AnyType(_) => this.alloc_type_expr(TypeExpr::Any),
//                 parsing::TypeExpr::UnitType(_) => this.alloc_type_expr(TypeExpr::Unit),
//                 parsing::TypeExpr::SelfType(_) => this.alloc_type_expr(TypeExpr::SelfTy),
//             })
//         }
//         inner(self, type_expr).unwrap_or_else(|| self.missing_type_expr())
//     }
//
//     fn type_expr_opt(&mut self, type_expr: Option<parsing::TypeExpr>) -> TypeExprId {
//         type_expr
//             .map(|ty| self.type_expr(ty))
//             .unwrap_or_else(|| self.missing_type_expr())
//     }
//
//     fn fn_type_param(&mut self, param: parsing::FnTypeParam) -> Option<FnTypeParam> {
//         let name = param.name().and_then(|n| n.text(self.source))?;
//         Some(FnTypeParam {
//             name: Symbol::new(self.db, name),
//             ty: self.type_expr_opt(param.ty()),
//         })
//     }
//
//     fn expr(&mut self, expr: parsing::Expr<'ast>) -> ExprId {
//         match expr {
//             parsing::Expr::AsExpr(as_expr) => self.missing_expr(),
//             parsing::Expr::IsExpr(is_expr) => self.missing_expr(),
//             parsing::Expr::IsNotExpr(is_not_expr) => self.missing_expr(),
//             parsing::Expr::SelfExpr(self_expr) => self.missing_expr(),
//             parsing::Expr::ClosureExpr(closure_expr) => self.missing_expr(),
//             parsing::Expr::FieldExpr(field_expr) => self.missing_expr(),
//             parsing::Expr::MethodExpr(method_expr) => self.missing_expr(),
//             parsing::Expr::RecordExpr(record_expr) => self.missing_expr(),
//             parsing::Expr::UnitExpr(unit_expr) => self.missing_expr(),
//             parsing::Expr::PathExpr(path_expr) => self.missing_expr(),
//             parsing::Expr::BinaryExpr(binary_expr) => self.missing_expr(),
//             parsing::Expr::UnaryExpr(unary_expr) => self.missing_expr(),
//             parsing::Expr::BlockExpr(block_expr) => {
//                 let stmts = block_expr
//                     .stmts()
//                     .filter_map(|stmt| self.stmt(stmt))
//                     .collect_vec();
//                 self.alloc_expr(Expr::Block { stmts })
//             }
//             parsing::Expr::IndexExpr(index_expr) => self.missing_expr(),
//             parsing::Expr::CallExpr(call_expr) => self.missing_expr(),
//             parsing::Expr::ParenExpr(paren_expr) => {
//                 let expr = self.expr_opt(paren_expr.expr());
//                 self.alloc_expr(Expr::Paren(expr))
//             }
//             parsing::Expr::ReturnExpr(return_expr) => {
//                 let expr = self.expr_opt(return_expr.expr());
//                 self.alloc_expr(Expr::Return { expr })
//             }
//             parsing::Expr::LitExpr(lit_expr) => self.alloc_expr(
//                 lit_expr
//                     .kind()
//                     .map(Expr::Lit)
//                     .unwrap_or_else(|| Expr::Missing),
//             ),
//             parsing::Expr::IfExpr(if_expr) => self.missing_expr(),
//         }
//     }
//
//     fn expr_opt(&mut self, expr: Option<parsing::Expr<'ast>>) -> ExprId {
//         expr.map(|e| self.expr(e))
//             .unwrap_or_else(|| self.missing_expr())
//     }
// }

// #[derive(Default)]
// pub struct BodyCtx {
//     exprs: Arena<Expr>,
//     pats: Arena<Pat>,
//     type_exprs: Arena<TypeExpr>,
//     stmts: Arena<Stmt>,
// }
//
struct LowerCtx<'db, 'ast, 's> {
    db: &'db dyn salsa::Database,
    source: &'s str,
    ast_file: parsing::File<'ast>,
    file: ide::File,
    root: Root,
    ast_id_map: AstIdMap,
}

impl<'db, 'ast, 's> LowerCtx<'db, 'ast, 's> {
    fn new(
        db: &'db dyn salsa::Database,
        source: &'s str,
        ast_file: parsing::File<'ast>,
        file: ide::File,
        root: Root,
    ) -> Self {
        Self {
            db,
            source,
            ast_file,
            file,
            root,
            ast_id_map: AstIdMap::new(),
        }
    }

    fn lower(mut self) -> (Vec<Item<'db>>, AstIdMap) {
        let items = self.ast_file.items().collect_vec();
        (self.items(items.into_iter()), self.ast_id_map)
    }

    fn items(&mut self, items_iter: impl Iterator<Item = parsing::Item<'ast>>) -> Vec<Item<'db>> {
        let mut items = vec![];
        for item in items_iter {
            match item {
                parsing::Item::FnItem(fn_item) => {
                    if let Some(fn_item) = self.fn_item(fn_item) {
                        items.push(Item::Function(fn_item));
                    }
                }
                parsing::Item::ModItem(mod_item) => {
                    if let Some(mod_item) = self.mod_item(mod_item) {
                        items.push(Item::Module(mod_item));
                    }
                }
                parsing::Item::ImplItem(impl_item) => {
                    if let Some(impl_item) = self.impl_item(impl_item) {
                        items.push(Item::Impl(impl_item));
                    }
                }
                parsing::Item::StructItem(struct_item) => {
                    if let Some(struct_item) = self.struct_item(struct_item) {
                        items.push(Item::Struct(struct_item));
                    }
                }
                parsing::Item::EnumItem(enum_item) => {
                    if let Some(enum_item) = self.enum_item(enum_item) {
                        items.push(Item::Enum(enum_item));
                    }
                }
                parsing::Item::UseItem(use_item) => {
                    if let Some(use_item) = self.use_item(use_item) {
                        items.push(Item::Use(use_item));
                    }
                }
            };
        }
        items
    }

    fn use_item(&mut self, use_item: parsing::UseItem<'ast>) -> Option<UseItem<'db>> {
        Some(UseItem::new(self.db, self.ast_id_map.insert(use_item)))
    }

    fn enum_item(&mut self, enum_item: parsing::EnumItem<'ast>) -> Option<Enum<'db>> {
        None
    }

    fn struct_item(&mut self, struct_item: parsing::StructItem<'ast>) -> Option<Struct<'db>> {
        todo!()
        // let name = struct_item.name().and_then(|n| n.text(self.source))?;
        // let parent = if let Some(parent) = struct_item.parent() {
        //     Some(self.path(parent.path()?)?)
        // } else {
        //     None
        // };
        // // let elems = struct_item
        // //     .elements()
        // //     .filter_map(|e| match e {
        // //         parsing::Elem::Field(field) => Some(Elem::Field(self.field(field))),
        // //         parsing::Elem::FnItem(fn_item) => {
        // //             self.fn_item(fn_item).map(|i| Elem::Function(i))
        // //         }
        // //     })
        // //     .collect_vec();
        // Some(Struct::new(
        //     self.db,
        //     Symbol::new(self.db, name),
        //     parent,
        //     // elems,
        //     self.ast_id_map.insert(struct_item),
        // ))
    }

    // fn field(&mut self, field: parsing::Field<'ast>) -> Field<'db> {
    //     let name = field.name().and_then(|n| n.text(self.source));
    //     let ty = field.ty().and_then(|ty| self.item_type_expr(ty));
    //     Field::new(self.db, name.map(|n| Symbol::new(self.db, n)), ty)
    // }

    fn impl_item(&mut self, impl_item: parsing::ImplItem<'ast>) -> Option<ImplBlock<'db>> {
        todo!()
        // let first = impl_item.first_type().and_then(|ty| self.type_expr(ty));
        // let second = impl_item.second_type().and_then(|ty| self.type_expr(ty));
        // let types = match (first, second) {
        //     (Some(inherent), None) => ImplTypes::Inherent(inherent),
        //     (Some(trait_ty), Some(impl_ty)) => ImplTypes::Trait { trait_ty, impl_ty },
        //     _ => return None,
        // };
        // let fn_items = impl_item
        //     .functions()
        //     .filter_map(|fn_item| self.fn_item(fn_item))
        //     .collect_vec();
        // Some(ImplBlock::new(
        //     self.db,
        //     types,
        //     fn_items,
        //     self.ast_id_map.insert(impl_item),
        // ))
    }

    fn fn_item(&mut self, fn_item: parsing::FnItem<'ast>) -> Option<Function<'db>> {
        todo!()
        // let name = fn_item.name().and_then(|n| n.text(self.source))?;
        // let mut params = vec![];
        // if let Some(param_list) = fn_item.params() {
        //     for param in param_list.params() {
        //         if param.self_token().is_some() {
        //             params.push(ItemFnParam::SelfParam);
        //             continue;
        //         }
        //         let pat = param.pattern().and_then(|p| self.pat(p));
        //         let type_expr = param.type_expr().and_then(|ty| self.type_expr(ty));
        //         params.push(ItemFnParam::PatParam { pat, type_expr });
        //     }
        // }
        //
        // let output = fn_item
        //     .output()
        //     .and_then(|o| o.ty().and_then(|ty| self.type_expr(ty)));
        //
        // Some(Function::new(
        //     self.db,
        //     Symbol::new(self.db, name),
        //     params,
        //     output,
        //     self.ast_id_map.insert(fn_item),
        // ))
    }

    fn mod_item(&mut self, mod_item: parsing::ModItem<'ast>) -> Option<Module<'db>> {
        let id = self.ast_id_map.insert(mod_item);
        let name = mod_item.name().and_then(|n| n.text(self.source))?;
        Some(Module::new(
            self.db,
            Symbol::new(self.db, name),
            match mod_item.semi() {
                Some(_) => ModuleKind::Declaration(id),
                None => ModuleKind::Definition(self.items(mod_item.items()).into()),
            },
            self.file,
        ))
    }

    // fn generic_param(&self, param: parsing::TypeParam<'ast>) -> Option<GenericParam<'db>> {
    //     Some(GenericParam::new(
    //         self.db,
    //         Symbol::new(self.db, param.name().and_then(|n| n.text(self.source))?),
    //         param
    //             .bounds()
    //             .filter_map(|b| self.type_expr(b))
    //             .collect_vec(),
    //     ))
    // }
    //
    // fn pat(&self, pat: parsing::Pattern<'ast>) -> Option<ItemPat<'db>> {
    //     Some(match pat {
    //         parsing::Pattern::NamePattern(name_pattern) => name_pattern
    //             .name()
    //             .and_then(|n| n.text(self.source))
    //             .map(|name| Symbol::new(self.db, name))
    //             .map(ItemPat::Name)?,
    //         parsing::Pattern::PathPattern(path_pattern) => {
    //             let path = path_pattern.path().and_then(|p| self.path(p));
    //             path.map(ItemPat::Path)?
    //         }
    //         parsing::Pattern::WildcardPattern(_) => ItemPat::Wildcard,
    //     })
    // }

    // fn path(&self, path: parsing::Path<'ast>) -> Option<Path<'db>> {
    //     let segments = path
    //         .segments()
    //         .filter_map(|s| self.path_segment(s))
    //         .collect_vec();
    //     if segments.is_empty() {
    //         return None;
    //     }
    //     Some(Path::new(self.db, segments))
    // }
    //
    // fn path_segment(&self, path_segment: parsing::PathSegment<'ast>) -> Option<PathSegment<'db>> {
    //     Some(PathSegment::new(
    //         self.db,
    //         Symbol::new(self.db, path_segment.ident(self.source)?),
    //         path_segment
    //             .generic_args()
    //             .map(|args| args.types())
    //             .into_iter()
    //             .flatten()
    //             .filter_map(|ty| self.type_expr(ty))
    //             .collect_vec(),
    //     ))
    // }

    // fn type_expr_from(&self, kind: TypeExprKind<'db>) -> TypeExpr<'db> {
    //     TypeExpr::new(self.db, kind)
    // }
    //
    // fn item_type_expr(
    //     &mut self,
    //     item_type_expr: parsing::ItemTypeExpr<'ast>,
    // ) -> Option<ItemTypeExpr<'db>> {
    //     Some(ItemTypeExpr::new(
    //         self.db,
    //         match item_type_expr {
    //             parsing::ItemTypeExpr::StructItemType(struct_item_type) => {
    //                 ItemTypeExprKind::Struct(self.struct_item(struct_item_type.struct_item()?)?)
    //             }
    //             parsing::ItemTypeExpr::EnumItemType(enum_item_type) => {
    //                 ItemTypeExprKind::Enum(self.enum_item(enum_item_type.enum_item()?)?)
    //             }
    //             parsing::ItemTypeExpr::ItemType(item_type) => {
    //                 ItemTypeExprKind::TypeExpr(self.type_expr(item_type.ty()?)?.kind(self.db))
    //             }
    //         },
    //     ))
    // }
    //
    // fn type_expr(&self, type_expr: parsing::TypeExpr<'ast>) -> Option<TypeExpr<'db>> {
    //     Some(match type_expr {
    //         parsing::TypeExpr::DynType(dyn_type) => {
    //             let path = self.path(dyn_type.path()?)?;
    //             self.type_expr_from(TypeExprKind::Dyn(path))
    //         }
    //         parsing::TypeExpr::ParenType(paren_type) => {
    //             let ty = paren_type.type_expr().and_then(|ty| self.type_expr(ty))?;
    //             self.type_expr_from(TypeExprKind::Paren(ty))
    //         }
    //         parsing::TypeExpr::PathType(path_type) => {
    //             let path = self.path(path_type.value()?)?;
    //             self.type_expr_from(TypeExprKind::Path(path))
    //         }
    //         parsing::TypeExpr::NilableType(nilable_type) => {
    //             let ty = nilable_type
    //                 .ty()
    //                 .and_then(|nilable| self.type_expr(nilable))?;
    //             self.type_expr_from(TypeExprKind::Nilable(ty))
    //         }
    //         parsing::TypeExpr::LitType(lit_type) => {
    //             self.type_expr_from(TypeExprKind::Lit(lit_type.kind()?))
    //         }
    //         parsing::TypeExpr::FnType(fn_type) => {
    //             let output = fn_type
    //                 .output()
    //                 .and_then(|o| o.ty().and_then(|ty| self.type_expr(ty)));
    //             let params = fn_type
    //                 .param_list()
    //                 .map(|p| p.params())
    //                 .into_iter()
    //                 .flatten()
    //                 .filter_map(|p| self.fn_type_param(p))
    //                 .collect_vec();
    //             self.type_expr_from(TypeExprKind::Fn {
    //                 params: FnTypeParamList::new(self.db, params),
    //                 output: output.map(|o| o.into()),
    //             })
    //         }
    //         parsing::TypeExpr::AnyType(_) => self.type_expr_from(TypeExprKind::Any),
    //         parsing::TypeExpr::UnitType(_) => self.type_expr_from(TypeExprKind::Unit),
    //         parsing::TypeExpr::SelfType(_) => self.type_expr_from(TypeExprKind::SelfTy),
    //     })
    // }

    // fn fn_type_param(&self, param: parsing::FnTypeParam<'ast>) -> Option<FnTypeParam<'db>> {
    //     let name = param.name().and_then(|n| n.text(self.source))?;
    //     Some(FnTypeParam::new(
    //         self.db,
    //         Symbol::new(self.db, name),
    //         self.type_expr(param.ty()?)?,
    //     ))
    // }
}

#[salsa::tracked]
pub fn items<'db>(db: &'db dyn salsa::Database, file: ide::File) -> Arc<Vec<Item<'db>>> {
    lower(db, file).0
}

#[salsa::tracked]
pub fn ast_map(db: &dyn salsa::Database, file: ide::File) -> Arc<AstIdMap> {
    lower(db, file).1
}

#[salsa::tracked]
fn lower<'db>(
    db: &'db dyn salsa::Database,
    file: ide::File,
) -> (Arc<Vec<Item<'db>>>, Arc<AstIdMap>) {
    let parse = file.parse(db);
    let ctx = LowerCtx::new(
        db,
        file.contents(db),
        parse.file(db).unwrap(),
        file,
        file.root(db),
    );
    let (items, id_map) = ctx.lower();
    (items.into(), id_map.into())
}
