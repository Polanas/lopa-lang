use std::{collections::HashMap, sync::Arc};

use itertools::{Itertools as _, concat};
use la_arena::{Arena, Idx};

use crate::{
    def::{AstIdMap, ItemMap, Symbol, UseTreeMap, ast_id_map, hir::*},
    ide::{self, File, Root},
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
//
//     fn pat_opt(&mut self, pat: Option<parsing::Pattern>) -> PatId {
//         pat.map(|e| self.pat(e))
//             .unwrap_or_else(|| self.missing_pat())
//     }
//
//
//     fn type_expr_opt(&mut self, type_expr: Option<parsing::TypeExpr>) -> TypeExprId {
//         type_expr
//             .map(|ty| self.type_expr(ty))
//             .unwrap_or_else(|| self.missing_type_expr())
//     }
//
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
struct Ctx<'db, 'ast, 's> {
    db: &'db dyn salsa::Database,
    source: &'s str,
    ast_file: parsing::File<'ast>,
    file: File,
    ast_id_map: AstIdMap,
}

impl<'db, 'ast, 's> Ctx<'db, 'ast, 's> {
    fn new(
        db: &'db dyn salsa::Database,
        source: &'s str,
        ast_file: parsing::File<'ast>,
        file: File,
    ) -> Self {
        Self {
            db,
            source,
            ast_file,
            file,
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
        use_item.use_keyword()?;
        use_item.use_tree()?;
        Some(UseItem::new(
            self.db,
            self.file,
            self.ast_id_map.insert(use_item),
        ))
    }

    fn enum_item(&mut self, enum_item: parsing::EnumItem<'ast>) -> Option<Enum<'db>> {
        enum_item.enum_token()?;
        let name = enum_item.name()?.text(self.source)?;
        let items = self.inner_items(enum_item.elements());

        Some(Enum::new(
            self.db,
            Symbol::new(self.db, name),
            self.file,
            items,
            self.ast_id_map.insert(enum_item),
        ))
    }

    fn struct_item(&mut self, struct_item: parsing::StructItem<'ast>) -> Option<Struct<'db>> {
        struct_item.struct_token()?;
        let name = struct_item.name()?.text(self.source)?;
        let items = self.inner_items(struct_item.elements());

        Some(Struct::new(
            self.db,
            Symbol::new(self.db, name),
            self.file,
            items,
            self.ast_id_map.insert(struct_item),
        ))
    }

    fn inner_items(
        &mut self,
        elems: impl Iterator<Item = parsing::Elem<'ast>>,
    ) -> Vec<InnerItem<'db>> {
        let mut items = vec![];
        for elem in elems {
            match elem {
                parsing::Elem::Field(field) => {
                    let Some(ty) = field.ty() else {
                        continue;
                    };
                    match ty {
                        parsing::ItemTypeExpr::StructItem(struct_item)
                            if let Some(item) = self.struct_item(struct_item) =>
                        {
                            items.push(InnerItem::Struct(item));
                        }
                        parsing::ItemTypeExpr::EnumItem(enum_item)
                            if let Some(item) = self.enum_item(enum_item) =>
                        {
                            items.push(InnerItem::Enum(item));
                        }
                        _ => {}
                    }
                }
                parsing::Elem::FnItem(fn_item) => {
                    if let Some(item) = self.fn_item(fn_item) {
                        items.push(InnerItem::Function(item));
                    }
                }
            }
        }
        items
    }

    fn impl_item(&mut self, impl_item: parsing::ImplItem<'ast>) -> Option<ImplBlock<'db>> {
        Some(ImplBlock::new(
            self.db,
            self.file,
            self.ast_id_map.insert(impl_item),
        ))
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
        let name = fn_item.name()?.text(self.source)?;
        Some(Function::new(
            self.db,
            Symbol::new(self.db, name),
            self.file,
            self.ast_id_map.insert(fn_item),
        ))
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
                Some(_) => ModuleKind::Declaration { id },
                None => ModuleKind::Definition {
                    id,
                    items: self.items(mod_item.items()).into(),
                },
            },
            self.file,
        ))
    }

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
}

struct ItemMapCtx<'db, 's> {
    db: &'db dyn salsa::Database,
    source: &'s str,
    map: ItemMap,
    file: File,
}

impl<'db, 's> ItemMapCtx<'db, 's> {
    fn new(db: &'db dyn salsa::Database, source: &'s str, file: File) -> Self {
        Self {
            db,
            file,
            source,
            map: ItemMap::default(),
        }
    }

    fn fn_contents(mut self, item: Function<'db>) -> Option<FunctionContents<'db>> {
        let ast_map = self.file.ast_map(self.db);
        let parse = self.file.parse(self.db);
        let ast_item =
            parse.cast::<parsing::FnItem>(self.db, ast_map.get(item.ast_ptr(self.db))?)?;
        let params = ast_item
            .params()
            .and_then(|p| self.fn_param_list(p))
            .unwrap_or_else(|| FnParamList::new(self.db, vec![]));
        let generics = ast_item
            .generics()
            .and_then(|g| self.generics(g))
            .unwrap_or_else(|| Generics::new(self.db, vec![]));
        let output = ast_item
            .output()
            .and_then(|o| o.ty())
            .and_then(|ty| self.type_expr(ty));

        Some(FunctionContents {
            item_map: self.map,
            params,
            generics,
            output,
        })
    }

    fn enum_item(mut self, item: Enum<'db>) -> Option<EnumContents<'db>> {
        let ast_map = self.file.ast_map(self.db);
        let parse = self.file.parse(self.db);
        let ast_item =
            parse.cast::<parsing::EnumItem>(self.db, ast_map.get(item.ast_ptr(self.db))?)?;
        let elems = ast_item
            .elements()
            .filter_map(|e| self.elem(item.inner_items(self.db).iter().cloned(), e))
            .collect_vec();

        Some(EnumContents {
            item_map: self.map,
            elems: ElemList::new(self.db, elems),
        })
    }

    fn struct_item(mut self, item: Struct<'db>) -> Option<StructContents<'db>> {
        let ast_map = self.file.ast_map(self.db);
        let parse = self.file.parse(self.db);
        let ast_item =
            parse.cast::<parsing::StructItem>(self.db, ast_map.get(item.ast_ptr(self.db))?)?;
        let parent = ast_item
            .parent()
            .and_then(|p| p.path())
            .and_then(|p| self.path(p));
        let elems = ast_item
            .elements()
            .filter_map(|e| self.elem(item.inner_items(self.db).iter().cloned(), e))
            .collect_vec();

        Some(StructContents {
            item_map: self.map,
            parent,
            elems: ElemList::new(self.db, elems),
        })
    }

    fn elem(
        &mut self,
        mut items: impl Iterator<Item = InnerItem<'db>>,
        elem: parsing::Elem,
    ) -> Option<Elem<'db>> {
        Some(match elem {
            parsing::Elem::Field(field) => {
                let name = field
                    .name()
                    .and_then(|n| n.text(self.source))
                    .map(|n| Symbol::new(self.db, n));
                let ty = field.ty().and_then(|ty| self.item_type_expr(items, ty));
                self.alloc_elem(ElemKind::Field(Field::new(self.db, name, ty)), elem)
            }
            parsing::Elem::FnItem(fn_item) => {
                let name = Symbol::new(self.db, fn_item.name().and_then(|n| n.text(self.source))?);
                let item = items.find(|i| i.name(self.db) == name)?;
                let InnerItem::Function(item) = item else {
                    return None;
                };
                self.alloc_elem(ElemKind::Function(item), elem)
            }
        })
    }

    fn item_type_expr(
        &mut self,
        mut items: impl Iterator<Item = InnerItem<'db>>,
        item_type_expr: parsing::ItemTypeExpr,
    ) -> Option<ItemTypeExpr<'db>> {
        Some(match item_type_expr {
            parsing::ItemTypeExpr::StructItem(struct_item) => {
                let name = Symbol::new(
                    self.db,
                    struct_item.name().and_then(|n| n.text(self.source))?,
                );
                let item = items.find(|i| i.name(self.db) == name)?;
                self.alloc_item_type_expr(
                    match item {
                        InnerItem::Struct(item) => ItemTypeExprKind::Struct(item),
                        InnerItem::Enum(item) => ItemTypeExprKind::Enum(item),
                        _ => return None,
                    },
                    item_type_expr,
                )
            }
            parsing::ItemTypeExpr::EnumItem(enum_item) => {
                let name =
                    Symbol::new(self.db, enum_item.name().and_then(|n| n.text(self.source))?);
                let item = items.find(|i| i.name(self.db) == name)?;
                self.alloc_item_type_expr(
                    match item {
                        InnerItem::Struct(item) => ItemTypeExprKind::Struct(item),
                        InnerItem::Enum(item) => ItemTypeExprKind::Enum(item),
                        _ => todo!(),
                    },
                    item_type_expr,
                )
            }
            parsing::ItemTypeExpr::TypeExpr(type_expr) => {
                let ty = self.type_expr(type_expr)?;
                self.alloc_item_type_expr(
                    ItemTypeExprKind::TypeExpr(ty),
                    parsing::ItemTypeExpr::TypeExpr(type_expr),
                )
            }
        })
    }

    fn fn_param_list(&mut self, param_list: parsing::ParamList) -> Option<FnParamList<'db>> {
        Some(FnParamList::new(
            self.db,
            param_list
                .params()
                .filter_map(|p| self.fn_param(p))
                .collect_vec(),
        ))
    }

    fn fn_param(&mut self, param: parsing::FnParam) -> Option<FnParam<'db>> {
        if param.self_token().is_some() {
            return Some(FnParam::new(self.db, FnParamKind::SelfParam));
        }
        let pat = param.pattern().and_then(|p| self.pat(p));
        let ty = param.type_expr().and_then(|ty| self.type_expr(ty));

        Some(FnParam::new(self.db, FnParamKind::Pat { pat, ty }))
    }

    fn type_expr(&mut self, type_expr: parsing::TypeExpr) -> Option<TypeExpr<'db>> {
        Some(match type_expr {
            parsing::TypeExpr::DynType(dyn_type) => {
                let path = self.path(dyn_type.path()?)?;
                self.alloc_type_expr(TypeExprKind::Dyn(path), type_expr)
            }
            parsing::TypeExpr::ParenType(paren_type) => {
                let ty = paren_type.type_expr().and_then(|ty| self.type_expr(ty))?;
                self.alloc_type_expr(TypeExprKind::Paren(ty), type_expr)
            }
            parsing::TypeExpr::PathType(path_type) => {
                let path = self.path(path_type.value()?)?;
                self.alloc_type_expr(TypeExprKind::Path(path), type_expr)
            }
            parsing::TypeExpr::NilableType(nilable_type) => {
                let ty = nilable_type
                    .ty()
                    .and_then(|nilable| self.type_expr(nilable))?;
                self.alloc_type_expr(TypeExprKind::Nilable(ty), type_expr)
            }
            parsing::TypeExpr::LitType(lit_type) => {
                self.alloc_type_expr(TypeExprKind::Lit(lit_type.kind()?), type_expr)
            }
            parsing::TypeExpr::FnType(fn_type) => {
                let output = fn_type
                    .output()
                    .and_then(|o| o.ty().and_then(|ty| self.type_expr(ty)));
                let params = fn_type
                    .param_list()
                    .map(|p| p.params())
                    .into_iter()
                    .flatten()
                    .map(|p| self.fn_type_param(p))
                    .collect_vec();
                self.alloc_type_expr(
                    TypeExprKind::Fn {
                        params: FnTypeParamList::new(self.db, params),
                        output,
                    },
                    type_expr,
                )
            }
            parsing::TypeExpr::AnyType(_) => self.alloc_type_expr(TypeExprKind::Any, type_expr),
            parsing::TypeExpr::UnitType(_) => self.alloc_type_expr(TypeExprKind::Unit, type_expr),
            parsing::TypeExpr::SelfType(_) => self.alloc_type_expr(TypeExprKind::SelfTy, type_expr),
            parsing::TypeExpr::TupleType(tuple_type) => {
                let types = tuple_type
                    .types()
                    .filter_map(|ty| self.type_expr(ty))
                    .collect_vec();
                self.alloc_type_expr(
                    TypeExprKind::Tuple(TupleType::new(self.db, types)),
                    type_expr,
                )
            }
        })
    }

    fn fn_type_param(&mut self, param: parsing::FnTypeParam) -> FnTypeParam<'db> {
        let name = param
            .name()
            .and_then(|n| n.text(self.source))
            .map(|n| Symbol::new(self.db, n));
        FnTypeParam::new(self.db, name, param.ty().and_then(|ty| self.type_expr(ty)))
    }

    fn generics(&mut self, generics: parsing::Generics) -> Option<Generics<'db>> {
        Some(Generics::new(
            self.db,
            generics
                .params()
                .filter_map(|p| self.generic_param(p))
                .collect_vec(),
        ))
    }

    fn generic_param(&mut self, param: parsing::TypeParam) -> Option<GenericParam<'db>> {
        Some(GenericParam::new(
            self.db,
            Symbol::new(self.db, param.name().and_then(|n| n.text(self.source))?),
            param
                .bounds()
                .filter_map(|b| self.type_expr(b))
                .collect_vec(),
        ))
    }

    fn pat(&mut self, pat: parsing::Pattern) -> Option<Pat<'db>> {
        Some(match pat {
            parsing::Pattern::NamePattern(name_pattern) => {
                let name = name_pattern
                    .name()
                    .and_then(|n| n.text(self.source))
                    .map(|name| Symbol::new(self.db, name))?;
                self.alloc_pat(PatKind::Name(name), pat)
            }
            parsing::Pattern::PathPattern(path_pattern) => {
                let path = path_pattern.path().and_then(|p| self.path(p))?;
                self.alloc_pat(PatKind::Path(path), pat)
            }
            parsing::Pattern::WildcardPattern(_) => self.alloc_pat(PatKind::Wildcard, pat),
        })
    }

    fn alloc_elem(&mut self, elem: ElemKind<'db>, elem_ast: parsing::Elem) -> Elem<'db> {
        let id = self.map.insert_elem(elem_ast);
        Elem::new(self.db, id, elem)
    }

    fn alloc_item_type_expr(
        &mut self,
        type_expr: ItemTypeExprKind<'db>,
        type_expr_ast: parsing::ItemTypeExpr,
    ) -> ItemTypeExpr<'db> {
        let id = self.map.insert_item_type_expr(type_expr_ast);
        ItemTypeExpr::new(self.db, id, type_expr)
    }

    fn alloc_type_expr(
        &mut self,
        type_expr: TypeExprKind<'db>,
        type_expr_ast: parsing::TypeExpr,
    ) -> TypeExpr<'db> {
        let id = self.map.insert_type_expr(type_expr_ast);
        TypeExpr::new(self.db, id, type_expr)
    }

    fn alloc_pat(&mut self, pat: PatKind<'db>, pat_ast: parsing::Pattern) -> Pat<'db> {
        let id = self.map.insert_pat(pat_ast);
        Pat::new(self.db, id, pat)
    }

    fn path(&mut self, path: parsing::Path) -> Option<Path<'db>> {
        let segments = path
            .segments()
            .filter_map(|s| self.path_segment(s))
            .collect_vec();
        if segments.is_empty() {
            return None;
        }
        Some(Path::new(self.db, segments))
    }

    fn path_segment(&mut self, path_segment: parsing::PathSegment) -> Option<PathSegment<'db>> {
        Some(PathSegment::new(
            self.db,
            Symbol::new(self.db, path_segment.ident(self.source)?),
            path_segment
                .generic_args()
                .map(|args| args.types())
                .into_iter()
                .flatten()
                .map(|ty| self.type_expr(ty))
                .collect_vec(),
        ))
    }
}

fn item_map_ctx<'db>(db: &'db dyn salsa::Database, file: File) -> ItemMapCtx<'db, 'db> {
    let source = file.contents(db);
    ItemMapCtx::new(db, source, file)
}

#[salsa::tracked]
impl<'db> Struct<'db> {
    #[salsa::tracked]
    pub fn contents(self, db: &'db dyn salsa::Database) -> StructContents<'db> {
        let ctx = item_map_ctx(db, self.file(db));
        ctx.struct_item(self).unwrap()
    }
}

#[salsa::tracked]
impl<'db> Enum<'db> {
    #[salsa::tracked]
    pub fn contents(self, db: &'db dyn salsa::Database) -> EnumContents<'db> {
        let ctx = item_map_ctx(db, self.file(db));
        ctx.enum_item(self).unwrap()
    }
}

#[salsa::tracked]
impl<'db> Function<'db> {
    #[salsa::tracked]
    pub fn contents(self, db: &'db dyn salsa::Database) -> FunctionContents<'db> {
        let ctx = item_map_ctx(db, self.file(db));
        ctx.fn_contents(self).unwrap()
    }
}

#[salsa::tracked]
impl<'db> UseItem<'db> {
    #[salsa::tracked]
    pub fn use_tree_map(self, db: &'db dyn salsa::Database, file: File) -> Option<Arc<UseTreeMap>> {
        self.use_tree_and_map(db, file).map(|(map, _)| map)
    }

    #[salsa::tracked]
    pub fn use_tree(self, db: &'db dyn salsa::Database, file: File) -> Option<UseTree> {
        self.use_tree_and_map(db, file).map(|(_, tree)| tree)
    }

    #[salsa::tracked]
    pub fn use_tree_and_map(
        self,
        db: &'db dyn salsa::Database,
        file: File,
    ) -> Option<(Arc<UseTreeMap>, UseTree)> {
        fn use_tree_inner<'a, 'db>(
            db: &'db dyn salsa::Database,
            use_tree: parsing::UseTree<'a>,
            map: &mut UseTreeMap,
            source: &str,
        ) -> Option<UseTree> {
            let id = map.insert(use_tree);
            Some(UseTree::new(
                db,
                match use_tree {
                    parsing::UseTree::UseRootPath(_) => UseTreeKind::Root,
                    parsing::UseTree::UseSuperPath(_) => UseTreeKind::Super,
                    parsing::UseTree::UseSelfName(_) => UseTreeKind::SelfUse,
                    parsing::UseTree::UseGlobal(_) => UseTreeKind::Global,
                    parsing::UseTree::UsePath(use_path) => {
                        let name = use_path
                            .name()
                            .and_then(|n| n.text(source))
                            .map(|t| Symbol::new(db, t))?;
                        let use_tree = use_tree_inner(db, use_path.use_tree()?, map, source)?;
                        UseTreeKind::Path { name, use_tree }
                    }
                    parsing::UseTree::UseName(use_name) => {
                        let name = use_name
                            .name()
                            .and_then(|n| n.text(source))
                            .map(|t| Symbol::new(db, t))?;
                        UseTreeKind::Name(name)
                    }
                    parsing::UseTree::UseTreeList(use_tree_list) => {
                        let use_trees = use_tree_list
                            .elements()
                            .filter_map(|tree| use_tree_inner(db, tree, map, source))
                            .collect_vec();
                        UseTreeKind::TreeList(UseTreeList::new(db, use_trees))
                    }
                },
                id,
            ))
        }
        let mut map = UseTreeMap::default();
        let parse = file.parse(db);
        let source = file.contents(db);

        let use_node_id = file.ast_map(db)[self.ast_ptr(db)];
        let use_tree = parse.cast::<parsing::UseTree>(db, use_node_id)?;
        let use_tree = use_tree_inner(db, use_tree, &mut map, source)?;

        Some((Arc::new(map), use_tree))
    }
}

#[salsa::tracked]
impl<'db> File {
    #[salsa::tracked]
    pub fn items(self, db: &'db dyn salsa::Database) -> Arc<Vec<Item<'db>>> {
        self.lower(db).0
    }

    #[salsa::tracked]
    pub fn ast_map(self, db: &dyn salsa::Database) -> Arc<AstIdMap> {
        self.lower(db).1
    }

    #[salsa::tracked]
    fn lower(self, db: &'db dyn salsa::Database) -> (Arc<Vec<Item<'db>>>, Arc<AstIdMap>) {
        let parse = self.parse(db);
        let ctx = Ctx::new(db, self.contents(db), parse.file(db).unwrap(), self);
        let (items, id_map) = ctx.lower();
        (items.into(), id_map.into())
    }
}
