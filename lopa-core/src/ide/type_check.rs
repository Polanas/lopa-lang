use std::{collections::HashMap, sync::Arc};

use itertools::Itertools;

use crate::{
    common::PathNodeType,
    def::{
        ExprId, HirNodeId, PatId, Symbol, SymbolList, TypeExprId,
        hir::{
            BodyMapSource, Expr, ExprKind, FnParam, Function, HirNode, ItemTypeExpr, Module, Pat,
            Stmt, TypeExpr, TypeExprKind,
        },
        mir::{BareFn, BareFnParam, BareFnParams, Type, TypeKind, TypeList},
    },
    hir_walker::{HirWalker, walk_type_expr},
    ide::{Diagnostic, ExprScopes, ModuleDef, PathMap},
};

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue, Hash)]
pub enum TypeSource<'db> {
    Expr(Expr<'db>),
    TypeExpr(TypeExpr<'db>),
    Pat(Pat<'db>),
}

#[derive(salsa::SalsaValue, PartialEq)]
pub struct TypeCheckMap<'db> {
    pub items: indexmap::IndexMap<TypeSource<'db>, Type<'db>>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue, Hash)]
pub enum ExpectedTypeGotValueSource<'db> {
    Module(Module<'db>),
    Function(Function<'db>),
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub enum TypeDiagnostic<'db> {
    TypeMismatch {
        expected: Type<'db>,
        actual: Type<'db>,
        expr: Expr<'db>,
    },
    UnknownType {
        id: HirNodeId,
        source: BodyMapSource<'db>,
        node_type: PathNodeType,
    },
    UnknownValue {
        name: Symbol,
    },
    NoSelf {
        type_expr: TypeExpr<'db>,
    },
    ExpectedTypeGotValue {
        source: ExpectedTypeGotValueSource<'db>,
    },
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
enum CheckedItem<'db> {
    Function(Function<'db>),
}

#[derive(Clone, PartialEq, salsa::SalsaValue)]
struct FunctionTypeContents<'db> {
    pub type_map: Arc<TypeCheckMap<'db>>,
    pub params: TypeList<'db>,
    pub output_type: Option<Type<'db>>,
}

// //NOTE: currently this only typechecks functions
// //TODO: make sure all Bodies are supported, including those attached to fields
struct TypeCheckCtx<'db> {
    db: &'db dyn salsa::Database,
    items: indexmap::IndexMap<TypeSource<'db>, Type<'db>>,
    checked_item: CheckedItem<'db>,
    self_ty: Option<Type<'db>>,
    path_map: Arc<PathMap<'db>>,
    scopes: Arc<ExprScopes<'db>>,
    diagnostics: Vec<TypeDiagnostic<'db>>,
}

impl<'db> HirWalker<'db> for TypeCheckCtx<'db> {
    fn walk_type_expr(&mut self, db: &'db dyn salsa::Database, type_expr: TypeExpr<'db>) {
        let ty = self.infer_type_expr(type_expr);
        self.items.insert(TypeSource::TypeExpr(type_expr), ty);
        walk_type_expr(db, self, type_expr);
    }
}

impl<'db> TypeCheckCtx<'db> {
    fn new(
        db: &'db dyn salsa::Database,
        checked_item: CheckedItem<'db>,
        self_ty: Option<Type<'db>>,
        path_map: Arc<PathMap<'db>>,
        scopes: Arc<ExprScopes<'db>>,
    ) -> Self {
        Self {
            db,
            checked_item,
            self_ty,
            path_map,
            scopes,
            diagnostics: Default::default(),
            items: Default::default(),
        }
    }

    fn check_expr(&mut self, expr: Expr<'db>, expected: Option<Type<'db>>) -> Type<'db> {
        todo!()
    }
    fn infer_expr(&mut self, expr: Expr<'db>) -> Type<'db> {
        match expr.kind(self.db) {
            ExprKind::Unit => Type::new(self.db, TypeKind::Unit),
            ExprKind::Lit(lit_kind) => todo!(),
            // ExprKind::Lit(lit_kind) => Type::new(self.db, TypeKind::Lit(lit_kind)),
            ExprKind::Path(path) => {
                let symbols = path.as_symbol_list(self.db);
                let Some(scope) = self.scopes.scope_for_expr(expr) else {
                    return Type::new(self.db, TypeKind::Unknown);
                };
                if symbols.symbols(self.db).len() == 1
                    && let Some(&name) = symbols.symbols(self.db).first()
                {
                    let Some(entry) = self.scopes.resolve_name_in_scope(scope, name) else {
                        self.error(TypeDiagnostic::UnknownValue { name });
                        return Type::new(self.db, TypeKind::Unknown);
                    };
                    return self
                        .pat_ty(TypeSource::Pat(entry.pat()))
                        .expect("pat should be resolved at this point");
                }
                //error is handled by the path map
                let Some(item) = self.path_map.items.get(&path.as_symbol_list(self.db)) else {
                    return Type::new(self.db, TypeKind::Unknown);
                };
                match item {
                    super::ResolveItem::Value(value) | super::ResolveItem::Both { value, .. } => {
                        match value {
                            super::ValueModuleDef::Function(item) => Type::new(
                                self.db,
                                TypeKind::Function {
                                    value: *item,
                                    generics: TypeList::new(self.db, []),
                                },
                            ),
                        }
                    }
                    super::ResolveItem::Type(_) => {
                        self.error(TypeDiagnostic::Unsupported);
                        Type::new(self.db, TypeKind::Unknown)
                    }
                }
            }
            ExprKind::As { expr, ty } => Type::new(self.db, TypeKind::Unknown),
            ExprKind::Is { expr, pat } => Type::new(self.db, TypeKind::Unknown),
            ExprKind::IsNot { expr, pat } => Type::new(self.db, TypeKind::Unknown),
            ExprKind::SelfExpr => Type::new(self.db, TypeKind::Unknown),
            ExprKind::Closure {
                params,
                body,
                output,
            } => Type::new(self.db, TypeKind::Unknown),
            ExprKind::Field { name, expr } => Type::new(self.db, TypeKind::Unknown),
            ExprKind::Method {
                expr,
                name,
                generic_args,
                args,
            } => Type::new(self.db, TypeKind::Unknown),
            ExprKind::Record { path, fields } => Type::new(self.db, TypeKind::Unknown),
            ExprKind::Binary { lhs, rhs, kind } => {
                let lhs = self.infer_expr(lhs);
                let rhs = self.infer_expr(rhs);
                todo!()
            }
            ExprKind::Unary { expr, kind } => Type::new(self.db, TypeKind::Unknown),
            ExprKind::Index { base, index } => Type::new(self.db, TypeKind::Unknown),
            ExprKind::Call { func, args } => Type::new(self.db, TypeKind::Unknown),
            ExprKind::Block { stmts } => {
                let mut ty = Type::new(self.db, TypeKind::Unit);
                for (position, stmt) in stmts.stmts(self.db).iter().with_position() {}
                ty
            }
            ExprKind::Paren(expr) => Type::new(self.db, TypeKind::Unknown),
            ExprKind::Return(expr) => Type::new(self.db, TypeKind::Unknown),
            ExprKind::If(if_expr) => Type::new(self.db, TypeKind::Unknown),
            ExprKind::Loop { block } => Type::new(self.db, TypeKind::Unknown),
            ExprKind::While { cond, block } => Type::new(self.db, TypeKind::Unknown),
            ExprKind::For {
                loop_expr,
                iterable,
                block,
            } => Type::new(self.db, TypeKind::Unknown),
            ExprKind::Tuple { exprs } => Type::new(self.db, TypeKind::Unknown),
        }
    }

    fn infer_stmt(&mut self, stmt: Stmt<'db>) -> Option<Type<'db>> {
        match stmt.kind(self.db) {
            crate::def::hir::StmtKind::Let { pat, ty, expr } => {
                //let x: int? = nil;
                todo!()
            }
            crate::def::hir::StmtKind::Expr { expr, semi } => {
                let ty = self.infer_expr(expr);
                Some(if semi.is_some() {
                    Type::new(self.db, TypeKind::Unit)
                } else {
                    ty
                })
            }
        }
    }

    fn infer_item_type_expr(&mut self, ty: ItemTypeExpr<'db>) -> Type<'db> {
        match ty.kind(self.db) {
            crate::def::hir::ItemTypeExprKind::TypeExpr(type_expr) => {
                self.infer_type_expr(type_expr)
            }
            crate::def::hir::ItemTypeExprKind::Struct(item) => Type::new(
                self.db,
                TypeKind::Struct {
                    value: item,
                    generics: TypeList::new(self.db, []),
                },
            ),
            crate::def::hir::ItemTypeExprKind::Enum(item) => Type::new(
                self.db,
                TypeKind::Enum {
                    value: item,
                    generics: TypeList::new(self.db, []),
                },
            ),
            crate::def::hir::ItemTypeExprKind::Class(item) => Type::new(
                self.db,
                TypeKind::Class {
                    value: item,
                    generics: TypeList::new(self.db, []),
                },
            ),
        }
    }

    fn infer_type_expr(&mut self, type_expr: TypeExpr<'db>) -> Type<'db> {
        match type_expr.kind(self.db) {
            TypeExprKind::Any => Type::new(self.db, TypeKind::Any),
            TypeExprKind::Unit => Type::new(self.db, TypeKind::Unit),
            TypeExprKind::Never => Type::new(self.db, TypeKind::Never),
            TypeExprKind::SelfTy => {
                if let Some(self_ty) = self.self_ty {
                    self_ty
                } else {
                    self.error(TypeDiagnostic::NoSelf { type_expr });
                    Type::new(self.db, TypeKind::Unknown)
                }
            }
            TypeExprKind::Tuple(types) | TypeExprKind::Dyn(types) => {
                let types = types
                    .types(self.db)
                    .iter()
                    .map(|ty| self.infer_type_expr(*ty))
                    .collect_vec();
                Type::new(self.db, TypeKind::Tuple(TypeList::new(self.db, types)))
            }
            TypeExprKind::Lit(lit_kind) => Type::new(self.db, TypeKind::Lit(lit_kind)),
            TypeExprKind::Path(path) => {
                self.infer_path(path.as_symbol_list(self.db), type_expr, PathNodeType::Type)
            }
            TypeExprKind::Nilable(type_expr) => Type::new(
                self.db,
                TypeKind::nilable(self.db, self.infer_type_expr(type_expr)),
            ),
            TypeExprKind::Paren(type_expr) => self.infer_type_expr(type_expr),
            TypeExprKind::Fn { params, output } => Type::new(
                self.db,
                TypeKind::BareFn(BareFn::new(
                    self.db,
                    BareFnParams::new(
                        self.db,
                        params
                            .params(self.db)
                            .iter()
                            .map(|p| {
                                BareFnParam::new(
                                    self.db,
                                    p.name(self.db),
                                    p.ty(self.db)
                                        .map(|ty| self.infer_type_expr(ty))
                                        .unwrap_or_else(|| Type::new(self.db, TypeKind::Unknown)),
                                )
                            })
                            .collect_vec(),
                    ),
                    output
                        .map(|o| self.infer_type_expr(o))
                        .unwrap_or_else(|| Type::new(self.db, TypeKind::Unit)),
                )),
            ),
        }
    }

    fn infer_path<H: HirNode<'db>>(
        &mut self,
        path: SymbolList,
        node: H,
        node_type: PathNodeType,
    ) -> Type<'db> {
        let Some(item) = self.path_map.items.get(&path) else {
            self.error(TypeDiagnostic::UnknownType {
                id: node.hir_id(self.db),
                source: node.body_map_source(self.db),
                node_type,
            });
            return Type::new(self.db, TypeKind::Unknown);
        };
        match item {
            super::ResolveItem::Type(ty) | super::ResolveItem::Both { ty, .. } => match ty {
                super::TypeModuleDef::Struct(item) => Type::new(
                    self.db,
                    TypeKind::Struct {
                        value: *item,
                        generics: TypeList::new(self.db, []),
                    },
                ),
                super::TypeModuleDef::Class(item) => Type::new(
                    self.db,
                    TypeKind::Class {
                        value: *item,
                        generics: TypeList::new(self.db, []),
                    },
                ),
                super::TypeModuleDef::Trait(item) => Type::new(
                    self.db,
                    TypeKind::Trait {
                        value: *item,
                        generics: TypeList::new(self.db, []),
                    },
                ),
                super::TypeModuleDef::Enum(item) => Type::new(
                    self.db,
                    TypeKind::Enum {
                        value: *item,
                        generics: TypeList::new(self.db, []),
                    },
                ),
                super::TypeModuleDef::Module(item) => {
                    self.error(TypeDiagnostic::ExpectedTypeGotValue {
                        source: ExpectedTypeGotValueSource::Module(*item),
                    });

                    Type::new(self.db, TypeKind::Unknown)
                }
            },
            super::ResolveItem::Value(def) => match def {
                super::ValueModuleDef::Function(item) => {
                    self.error(TypeDiagnostic::ExpectedTypeGotValue {
                        source: ExpectedTypeGotValueSource::Function(*item),
                    });
                    Type::new(self.db, TypeKind::Unknown)
                }
            },
        }
    }

    fn error(&mut self, diagnostic: TypeDiagnostic<'db>) {
        self.diagnostics.push(diagnostic);
    }

    fn insert_expr_ty(&mut self, expr: Expr<'db>, ty: Type<'db>) {
        self.items.insert(TypeSource::Expr(expr), ty);
    }

    fn insert_pat_ty(&mut self, pat: Pat<'db>, ty: Type<'db>) {
        self.items.insert(TypeSource::Pat(pat), ty);
    }

    fn pat_ty(&self, pat: TypeSource<'db>) -> Option<Type<'db>> {
        self.items.get(&pat).cloned()
    }

    fn expr_ty(&self, expr: TypeSource<'db>) -> Option<Type<'db>> {
        self.items.get(&expr).cloned()
    }
}

#[salsa::tracked]
impl<'db> Function<'db> {
    #[salsa::tracked(returns(ref))]
    fn type_contents(self, db: &'db dyn salsa::Database) -> FunctionTypeContents<'db> {
        // let ctx = TypeCheckCtx {
        //     db,
        //     items: todo!(),
        //     checked_item: CheckedItem::Function(self),
        //     self_ty: None,
        //     path_map: self.path_map(db),
        //     scopes: todo,
        //     diagnostics: todo!(),
        // };
        FunctionTypeContents {
            type_map: todo!(),
            params: todo!(),
            output_type: todo!(),
        }
    }
}

mod test {
    // fn infer_types(source: &str) -> String {
    //
    // }
}

// #[salsa::tracked]
// impl<'db> Function<'db> {
//     #[salsa::tracked(returns(ref))]
//     fn type_check(self, db: &'db dyn salsa::Database) -> TypeCheckMap<'db> {
//         let mut ctx = TypeCheckCtx {
//             db,
//             func: self,
//             pattern_ty_map: Default::default(),
//             expr_ty_map: Default::default(),
//             diagnostics: Default::default(),
//         };
//         ctx.check_fn();
//         TypeCheckMap {
//             pattern_ty_map: ctx.pattern_ty_map,
//             expr_ty_map: ctx.expr_ty_map,
//             diagnostics: ctx.diagnostics,
//         }
//     }
// }
//
//
// impl<'db> TypeCheckCtx<'db> {
//     fn check_fn(&mut self) {
//         for param in self.func.contents(self.db).params.params(self.db) {
//             self.check_param(param);
//         }
//     }
//
//     fn check_param(&mut self, param: &FnParam<'db>) {
//         match param.kind(self.db) {
//             rate::def::hir::FnParamKind::Pat { ty, pat, expr } => {
//                 if let Some(ty) = ty
//                     && let Some(pat) = pat
//                 {
//                     //TODO: verify expression type
//                     //Also, checking that the pattern conforms the type needs to happen here
//                     // self.pattern_ty_map.insert(pat.id(self.db), ty)
//                 }
//             }
//             crate::def::hir::FnParamKind::SelfParam => {}
//         }
//     }
//
//     fn check_expr(&mut self, expr: Expr<'db>) -> Option<Type<'db>> {
//         if let Some(ty) = self.expr_ty(expr.id(self.db)) {
//             return Some(ty);
//         };
//         match expr.kind(self.db) {
//             crate::def::hir::ExprKind::Unit => {
//                 let ty = Type::new(self.db, TypeKind::Unit);
//                 self.insert_expr_ty(expr, ty);
//                 Some(ty)
//             }
//             crate::def::hir::ExprKind::Lit(lit_kind) => todo!(),
//             crate::def::hir::ExprKind::Path(path) => todo!(),
//             crate::def::hir::ExprKind::As { expr, ty } => todo!(),
//             crate::def::hir::ExprKind::Is { expr, pat } => todo!(),
//             crate::def::hir::ExprKind::IsNot { expr, pat } => todo!(),
//             crate::def::hir::ExprKind::SelfExpr => todo!(),
//             crate::def::hir::ExprKind::Closure {
//                 params,
//                 body,
//                 output,
//             } => todo!(),
//             crate::def::hir::ExprKind::Field { name, expr } => todo!(),
//             crate::def::hir::ExprKind::Method {
//                 expr,
//                 name,
//                 generic_args,
//                 args,
//             } => todo!(),
//             crate::def::hir::ExprKind::Record { path, fields } => todo!(),
//             crate::def::hir::ExprKind::Binary { lhs, rhs, kind } => todo!(),
//             crate::def::hir::ExprKind::Unary { expr, kind } => todo!(),
//             crate::def::hir::ExprKind::Index { base, index } => todo!(),
//             crate::def::hir::ExprKind::Call { func, args } => todo!(),
//             crate::def::hir::ExprKind::Block { stmts } => todo!(),
//             crate::def::hir::ExprKind::Paren(expr) => todo!(),
//             crate::def::hir::ExprKind::Return(expr) => todo!(),
//             crate::def::hir::ExprKind::If(if_expr) => todo!(),
//             crate::def::hir::ExprKind::Loop { block } => todo!(),
//             crate::def::hir::ExprKind::While { cond, block } => todo!(),
//             crate::def::hir::ExprKind::For {
//                 loop_expr,
//                 iterable,
//                 block,
//             } => todo!(),
//             crate::def::hir::ExprKind::Tuple { exprs } => todo!(),
//         }
//     }
//
// }
//
// fn path_type<'db>(
//     db: &'db dyn salsa::Database,
//     path: SymbolList,
//     path_map: &PathMap<'db>,
// ) -> Result<Type<'db>, Diagnostic> {
//     // let Some(item) = path_map.items.get(&path) else {
//     //     return Err(Diagnostic {
//     //         message: format!("cannot find type `{}` in this scope"),
//     //         location: todo!(),
//     //         kind: todo!(),
//     //     });
//     // };
//     todo!()
// }
