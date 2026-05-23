// use std::{ops::Deref, sync::Arc, time::Duration};
//
// use super::Type;
// use itertools::Itertools;
// use la_arena::ArenaMap;
// use rowan::TextRange;
// use ustr::Ustr;
//
// use crate::{
//     common::LitKind,
//     def::{
//         self, body,
//         ir::{self, ExprId, PatternId, Stmt},
//         resolver, scope,
//     },
//     ide::{self, diagnostics, source_map},
//     parsing::ast::{SyntaxNode, SyntaxNodePtr},
//     ptr::Ptr,
// };
//
// #[derive(Debug, Clone, PartialEq, Eq, salsa::Update)]
// pub struct InferenceResult {
//     pub pattern_ty_map: ArenaMap<PatternId, Type>,
//     pub expr_ty_map: ArenaMap<ExprId, Type>,
//     pub diagnostics: Vec<TypeDiagnostic>,
// }
//
// impl InferenceResult {
//     pub fn ty_for_pattern(&self, pattern: PatternId) -> Type {
//         self.pattern_ty_map[pattern].clone()
//     }
//
//     pub fn ty_for_expr(&self, expr: ExprId) -> Type {
//         self.expr_ty_map[expr].clone()
//     }
// }
//
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeErrorKind {
    pub message: String,
}
//
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum TypeDiagnostic {
//     TypeMismatch {
//         expected: Type,
//         actual: Type,
//         expr: ExprId,
//     },
//     UnknownValue {
//         expr: ExprId,
//     },
// }
//
// struct InferCtx<'db> {
//     db: &'db dyn salsa::Database,
//     body: &'db def::body::Body,
//     func: ir::Function<'db>,
//     scopes: &'db scope::ExprScopes,
//     diagnostics: Ptr<Vec<TypeDiagnostic>>,
//     pattern_ty_map: Ptr<ArenaMap<PatternId, Type>>,
//     expr_ty_map: Ptr<ArenaMap<ExprId, Type>>,
// }
//
// impl<'db> InferCtx<'db> {
//     fn results(self) -> InferenceResult {
//         InferenceResult {
//             pattern_ty_map: Ptr::try_unwrap(self.pattern_ty_map).unwrap(),
//             expr_ty_map: Ptr::try_unwrap(self.expr_ty_map).unwrap(),
//             diagnostics: Ptr::try_unwrap(self.diagnostics).unwrap(),
//         }
//     }
//
//     fn infer_function(&mut self) {
//         self.infer_expr(self.body.body_expr());
//     }
//
//     fn insert_pattern_ty(&self, pattern_id: PatternId, ty: Type) {
//         self.pattern_ty_map.clone().insert(pattern_id, ty);
//     }
//
//     fn expr_ty(&self, expr_id: ExprId) -> Option<&Type> {
//         self.infer_expr(expr_id);
//         self.expr_ty_map.get(expr_id)
//     }
//
//     fn infer_expr(&self, expr_id: ExprId) -> Option<ExprId> {
//         let mut expr_ty_map = self.expr_ty_map.clone();
//         if expr_ty_map.contains_idx(expr_id) {
//             return Some(expr_id);
//         };
//         match self.body.expr(expr_id) {
//             ir::Expr::Missing => return None,
//             ir::Expr::Unit => {
//                 expr_ty_map.insert(expr_id, Type::Unit);
//             }
//             ir::Expr::Path(ustr) => {
//                 todo!()
//                 //TODO: not use resolvers here
//                 // let resolver = resolver::resolver_for_expr(self.db, self.func, expr_id);
//                 // let Some(result) = resolver.resolve_name(ustr) else {
//                 //     self.add_error(TypeDiagnostic::UnknownValue { expr: expr_id });
//                 //     return None;
//                 // };
//                 // match result {
//                 //     resolver::ResolveResult::Local(local) => {
//                 //         let Some(pattern_ty) = self.pattern_ty_map.get(local.pattern_id) else {
//                 //             self.add_error(TypeDiagnostic::UnknownValue { expr: expr_id });
//                 //             return None;
//                 //         };
//                 //         expr_ty_map.insert(expr_id, pattern_ty.clone());
//                 //     }
//                 //     resolver::ResolveResult::Function(function) => {
//                 //         expr_ty_map.insert(expr_id, function_type(self.db, function).clone());
//                 //     }
//                 // };
//             }
//             ir::Expr::Lit(lit_kind) => {
//                 expr_ty_map.insert(expr_id, Type::Lit(*lit_kind));
//             }
//             ir::Expr::BlockExpr { stmts } => {
//                 for stmt in stmts {
//                     self.infer_stmt(stmt);
//                 }
//             }
//             ir::Expr::If {
//                 if_cond,
//                 if_branch,
//                 else_branch,
//             } => {}
//             ir::Expr::Unary { expr, kind } => {}
//             ir::Expr::Binary { left, right, kind } => {}
//             ir::Expr::Return { expr } => {}
//             ir::Expr::Index { base, index } => {}
//             ir::Expr::Call { func, args } => {}
//             ir::Expr::Paren { expr } => {}
//         };
//         Some(expr_id)
//     }
//
//     fn infer_stmt(&self, stmt: &Stmt) -> Option<()> {
//         match stmt {
//             Stmt::Let {
//                 pattern: pattern_id,
//                 ty,
//                 expr: expr_id,
//             } => {
//                 let infer_ty = self.expr_ty(*expr_id)?;
//                 let ty = ty.as_ref().map(type_from_type_expr);
//
//                 if let Some(ty) = ty {
//                     if !self.can_assign_type(&ty, infer_ty) {
//                         self.add_error(TypeDiagnostic::TypeMismatch {
//                             expected: ty.clone(),
//                             actual: infer_ty.clone(),
//                             expr: *expr_id,
//                         });
//                     }
//                     self.insert_pattern_ty(*pattern_id, ty);
//                 }
//                 self.insert_pattern_ty(*pattern_id, infer_ty.clone());
//             }
//             Stmt::Expr { expr, semi } => {}
//         };
//         Some(())
//     }
//
//     fn can_assign_type(&self, lhs: &Type, rhs: &Type) -> bool {
//         match (lhs, rhs) {
//             (Type::Nilable(lhs), Type::Nilable(rhs)) => self.can_assign_type(lhs, rhs),
//             (Type::Nilable(lhs), rhs) => {
//                 matches!(lhs.deref(), Type::Lit(LitKind::Nil)) || self.can_assign_type(lhs, rhs)
//             }
//             (rhs, Type::Nilable(_)) if !rhs.nilable() => false,
//             (Type::Any, _) | (_, Type::Any) => true,
//             (Type::Lit(lhs), Type::Lit(rhs)) => {
//                 rhs == lhs || (*lhs == LitKind::Float && *rhs == LitKind::Int)
//             }
//             (Type::Unit, Type::Unit) => true,
//             _ => false,
//         }
//     }
//
//     fn add_error(&self, diagnostic: TypeDiagnostic) {
//         self.diagnostics.clone().push(diagnostic);
//     }
// }
//
// fn type_from_type_expr(ty: &ir::TypeExpr) -> Type {
//     match ty {
//         ir::TypeExpr::PathType(_) => Type::Unknown,
//         ir::TypeExpr::Unknown => Type::Unknown,
//         ir::TypeExpr::Nilable(nilable_type) => {
//             Type::Nilable(type_from_type_expr(&nilable_type.value).into())
//         }
//         ir::TypeExpr::Lit(lit_type) => Type::Lit(lit_type.kind),
//         ir::TypeExpr::Any => Type::Any,
//         ir::TypeExpr::Unit => Type::Unit,
//     }
// }
//
// fn type_to_string(ty: &Type) -> Ustr {
//     match ty {
//         Type::Unknown => Ustr::from("unknown"),
//         Type::Unit => Ustr::from("()"),
//         Type::Any => Ustr::from("any"),
//         Type::Nilable(nilable) => Ustr::from(&format!("{}", type_to_string(&nilable))),
//         Type::Lit(lit_kind) => match lit_kind {
//             LitKind::Float => Ustr::from("float"),
//             LitKind::Int => Ustr::from("int"),
//             LitKind::String => Ustr::from("string"),
//             LitKind::Bool => Ustr::from("bool"),
//             LitKind::Nil => Ustr::from("nil"),
//         },
//         Type::Function {
//             params,
//             return_type,
//         } => Ustr::from("todo: fn"),
//     }
// }
//
// #[salsa::tracked(returns(ref))]
// pub fn function_type<'db>(db: &'db dyn salsa::Database, func: ir::Function<'db>) -> Type {
//     let body = ide::body(db, func);
//     Type::Function {
//         params: body
//             .params()
//             .iter()
//             .zip(func.params(db).iter())
//             .map(|(body_param, param)| {
//                 let pattern = body.pattern(body_param.pattern);
//                 let name = match pattern {
//                     ir::Pattern::Missing => None,
//                     ir::Pattern::Name(ustr) => Some(*ustr),
//                 };
//
//                 (name, type_from_type_expr(param.ty(db)))
//             })
//             .collect_vec(),
//         return_type: Arc::new(Type::Unknown),
//     }
// }
//
// fn expr_node<'db>(
//     db: &'db dyn salsa::Database,
//     func: ir::Function<'db>,
//     expr: ExprId,
// ) -> Option<SyntaxNode> {
//     let source_map = ide::source_map(db, func);
//     let parse = ide::parse(db, func.file(db));
//     source_map
//         .node_for_expr(expr)
//         .map(|n| n.value.syntax_node_ptr().to_node(&parse.syntax_node(db)))
// }
//
// fn expr_range<'db>(
//     db: &'db dyn salsa::Database,
//     func: ir::Function<'db>,
//     expr: ExprId,
// ) -> Option<TextRange> {
//     expr_node(db, func, expr).map(|t| t.text_range())
// }
//
// fn expr_text<'db>(
//     db: &'db dyn salsa::Database,
//     func: ir::Function<'db>,
//     expr: ExprId,
// ) -> Option<Ustr> {
//     expr_node(db, func, expr).map(|n| n.text().to_string().into())
// }
//
// fn pattern_node<'db>(
//     db: &'db dyn salsa::Database,
//     func: ir::Function<'db>,
//     pattern: PatternId,
// ) -> Option<SyntaxNode> {
//     let source_map = ide::source_map(db, func);
//     let parse = ide::parse(db, func.file(db));
//     source_map
//         .node_for_pattern(pattern)
//         .map(|n| n.value.syntax_node_ptr().to_node(&parse.syntax_node(db)))
// }
//
// fn pattern_range<'db>(
//     db: &'db dyn salsa::Database,
//     func: ir::Function<'db>,
//     pattern: PatternId,
// ) -> Option<TextRange> {
//     pattern_node(db, func, pattern).map(|t| t.text_range())
// }
//
// fn pattern_text<'db>(
//     db: &'db dyn salsa::Database,
//     func: ir::Function<'db>,
//     pattern: PatternId,
// ) -> Option<Ustr> {
//     pattern_node(db, func, pattern).map(|n| n.text().to_string().into())
// }
//
// #[salsa::tracked]
// pub fn type_diagnostics<'db>(
//     db: &'db dyn salsa::Database,
//     func: ir::Function<'db>,
// ) -> Vec<(String, TextRange)> {
//     let result = infer_function(db, func);
//     result
//         .diagnostics
//         .iter()
//         .map(|d| match d {
//             TypeDiagnostic::TypeMismatch {
//                 expected,
//                 actual,
//                 expr,
//             } => (
//                 format!(
//                     "expected {}, got {}",
//                     type_to_string(expected),
//                     type_to_string(actual)
//                 ),
//                 expr_range(db, func, *expr).unwrap(),
//             ),
//             TypeDiagnostic::UnknownValue { expr } => (
//                 format!(
//                     "cannot find value `{}` in this scope",
//                     expr_text(db, func, *expr).unwrap()
//                 ),
//                 expr_range(db, func, *expr).unwrap(),
//             ),
//         })
//         .collect_vec()
// }
//
// #[salsa::tracked(returns(ref))]
// pub fn infer_function<'db>(
//     db: &'db dyn salsa::Database,
//     func: ir::Function<'db>,
// ) -> InferenceResult {
//     let body = ide::body(db, func);
//     let scopes = scope::expr_scopes(db, func);
//     let mut ctx = InferCtx {
//         db,
//         body,
//         func,
//         diagnostics: Ptr::new(Default::default()),
//         pattern_ty_map: Ptr::new(Default::default()),
//         expr_ty_map: Ptr::new(Default::default()),
//         scopes,
//     };
//     ctx.infer_function();
//     ctx.results()
// }
