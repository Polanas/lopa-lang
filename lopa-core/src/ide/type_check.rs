use std::{collections::HashMap, sync::Arc};

use itertools::Itertools;

use crate::{
    common::PathNodeType,
    def::{
        ExprId, HirNodeId, PatId, Symbol, SymbolList, TypeExprId,
        hir::{BodyMapSource, Expr, FnParam, Function, HirNode, Pat, Stmt, TypeExpr},
        mir::{Type, TypeKind, TypeList},
    },
    hir_walker::{HirWalker, walk_type_expr},
    ide::{Diagnostic, ModuleDef, PathMap},
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
    NoSelf {
        type_expr: TypeExpr<'db>,
    },
    ExpectedTypeGotFn {
        name: Symbol,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
enum CheckedItem<'db> {
    Function(Function<'db>),
}

// //NOTE: currently this only typechecks functions
// //TODO: make sure all Bodies are supported, including those attached to fields
struct TypeCheckCtx<'db> {
    db: &'db dyn salsa::Database,
    items: indexmap::IndexMap<TypeSource<'db>, Type<'db>>,
    checked_item: CheckedItem<'db>,
    self_ty: Option<Type<'db>>,
    path_map: Arc<PathMap<'db>>,
    diagnostics: Vec<TypeDiagnostic<'db>>,
}

impl<'db> HirWalker<'db> for TypeCheckCtx<'db> {
    // fn walk_type_expr(
    //     &mut self,
    //     db: &'db dyn salsa::Database,
    //     type_expr: crate::def::hir::TypeExpr<'db>,
    // ) {
    //     match type_expr.kind(db) {
    //         crate::def::hir::TypeExprKind::Any => todo!(),
    //         crate::def::hir::TypeExprKind::Unit => todo!(),
    //         crate::def::hir::TypeExprKind::Never => todo!(),
    //         crate::def::hir::TypeExprKind::SelfTy => todo!(),
    //         crate::def::hir::TypeExprKind::Tuple(type_expr_list) => todo!(),
    //         crate::def::hir::TypeExprKind::Lit(lit_kind) => todo!(),
    //         crate::def::hir::TypeExprKind::Path(path) => todo!(),
    //         crate::def::hir::TypeExprKind::Dyn(type_expr_list) => todo!(),
    //         crate::def::hir::TypeExprKind::Nilable(type_expr) => todo!(),
    //         crate::def::hir::TypeExprKind::Paren(type_expr) => todo!(),
    //         crate::def::hir::TypeExprKind::Fn { params, output } => todo!(),
    //     };
    //     walk_type_expr(db, self, type_expr);
    // }
}

impl<'db> TypeCheckCtx<'db> {
    fn resolve_type_expr(&mut self, type_expr: TypeExpr<'db>) -> Type<'db> {
        match type_expr.kind(self.db) {
            crate::def::hir::TypeExprKind::Any => Type::new(self.db, TypeKind::Any),
            crate::def::hir::TypeExprKind::Unit => Type::new(self.db, TypeKind::Unit),
            crate::def::hir::TypeExprKind::Never => Type::new(self.db, TypeKind::Never),
            crate::def::hir::TypeExprKind::SelfTy => {
                if let Some(self_ty) = self.self_ty {
                    self_ty
                } else {
                    self.error(TypeDiagnostic::NoSelf { type_expr });
                    Type::new(self.db, TypeKind::Unknown)
                }
            }
            crate::def::hir::TypeExprKind::Tuple(type_expr_list) => {
                let types = type_expr_list
                    .types(self.db)
                    .iter()
                    .map(|ty| self.resolve_type_expr(*ty))
                    .collect_vec();
                Type::new(self.db, TypeKind::Tuple(TypeList::new(self.db, types)))
            }
            crate::def::hir::TypeExprKind::Lit(lit_kind) => {
                Type::new(self.db, TypeKind::Lit(lit_kind))
            }
            crate::def::hir::TypeExprKind::Path(path) => {
                self.resolve_path(path.as_symbol_list(self.db), type_expr, PathNodeType::Type)
            }
            crate::def::hir::TypeExprKind::Dyn(type_expr_list) => todo!(),
            crate::def::hir::TypeExprKind::Nilable(type_expr) => todo!(),
            crate::def::hir::TypeExprKind::Paren(type_expr) => todo!(),
            crate::def::hir::TypeExprKind::Fn { params, output } => todo!(),
        }
    }

    fn resolve_path<H: HirNode<'db>>(
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
        // match item {
        //     super::ResolveItem::Type(ty) | super::ResolveItem::Both { ty, .. } => match ty {
        //         super::ModuleDef::Function(function) => todo!(),
        //         super::ModuleDef::Struct(_) => todo!(),
        //         super::ModuleDef::Class(class) => todo!(),
        //         super::ModuleDef::Trait(_) => todo!(),
        //         super::ModuleDef::Enum(_) => todo!(),
        //         super::ModuleDef::Module(module) => todo!(),
        //     },
        //     super::ResolveItem::Value(module_def) => {
        //         match module_def {
        //             ModuleDef::Function(function) => todo!(),
        //             ModuleDef::Struct(_) => todo!(),
        //             ModuleDef::Class(class) => todo!(),
        //             ModuleDef::Trait(_) => todo!(),
        //             ModuleDef::Enum(_) => todo!(),
        //             ModuleDef::Module(module) => todo!(),
        //         }
        //         let ModuleDef::Function(func) = *module_def else {
        //             unreachable!();
        //         };
        //         self.error(TypeDiagnostic::ExpectedTypeGotFn {
        //             name: func.name(self.db),
        //         });
        //         Type::new(self.db, TypeKind::Unknown)
        //     }
        // }
        todo!()
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
//             crate::def::hir::FnParamKind::Pat { ty, pat, expr } => {
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
