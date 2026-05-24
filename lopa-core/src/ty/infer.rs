use std::{borrow::Cow, ops::Deref, sync::Arc, time::Duration};

use super::Type;
use itertools::{Itertools, Position};
use la_arena::{ArenaMap, Idx};
use notify_rust::Notification;
use rowan::TextRange;
use ustr::Ustr;

use crate::{
    common::LitKind,
    def::{
        self, body,
        ir::{self, Expr, ExprId, PatternId, Stmt, TypeExpr},
        resolver, scope,
    },
    ide::{self, diagnostics, source_map},
    parsing::{
        ast::{SyntaxNode, SyntaxNodePtr},
        lexer,
    },
    ptr::Ptr,
    ty::{self, BareFn},
};

#[derive(Debug, Clone, PartialEq, Eq, salsa::Update)]
pub struct InferenceResult<'db> {
    pub pattern_ty_map: ArenaMap<PatternId, Type<'db>>,
    pub expr_ty_map: ArenaMap<Idx<Expr<'db>>, Type<'db>>,
    pub diagnostics: Vec<TypeDiagnostic<'db>>,
}

impl<'db> InferenceResult<'db> {
    pub fn ty_for_pattern(&'_ self, pattern: PatternId) -> &'_ Type<'_> {
        &self.pattern_ty_map[pattern]
    }

    pub fn ty_for_expr(&'_ self, expr: ExprId) -> &'_ Type<'_> {
        &self.expr_ty_map[expr.into()]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeErrorKind {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::Update)]
pub enum TypeDiagnostic<'db> {
    TypeMismatch {
        expected: Type<'db>,
        actual: Type<'db>,
        expr: ExprId,
    },
    UnknownValue {
        expr: ExprId,
    },
}

struct InferCtx<'db> {
    db: &'db dyn salsa::Database,
    body: &'db def::body::Body<'db>,
    func: ir::Function<'db>,
    scopes: &'db scope::ExprScopes<'db>,
    diagnostics: Vec<TypeDiagnostic<'db>>,
    pattern_ty_map: ArenaMap<PatternId, Type<'db>>,
    pub expr_ty_map: ArenaMap<Idx<Expr<'db>>, Type<'db>>,
}

impl<'db> InferCtx<'db> {
    fn results(self) -> InferenceResult<'db> {
        InferenceResult {
            pattern_ty_map: self.pattern_ty_map,
            expr_ty_map: self.expr_ty_map,
            diagnostics: self.diagnostics,
        }
    }

    fn infer_function(&mut self) {
        self.infer_expr(self.body.body_expr());
    }

    fn insert_pattern_ty(&mut self, pattern_id: PatternId, ty: Type<'db>) {
        self.pattern_ty_map.insert(pattern_id, ty);
    }

    fn expr_ty(&mut self, expr_id: ExprId) -> Option<&Type> {
        self.infer_expr(expr_id);
        self.expr_ty_map.get(expr_id.into())
    }

    //TODO: add expected type
    fn infer_expr(&mut self, expr_id: ExprId) -> Option<ExprId> {
        if self.expr_ty_map.contains_idx(expr_id.into()) {
            return Some(expr_id);
        };
        match self.body.expr(expr_id) {
            ir::Expr::Missing => return None,
            ir::Expr::Unit => {
                self.expr_ty_map.insert(expr_id.into(), Type::Unit);
            }
            ir::Expr::Path(ustr) => {
                let result = resolver::resolve_name_for_expr(self.db, expr_id, self.func, &ustr[0]);
                let Some(result) = result else {
                    self.add_error(TypeDiagnostic::UnknownValue { expr: expr_id });
                    return None;
                };
                match result {
                    resolver::ResolveResult::Local(local) => {
                        let Some(pattern_ty) = self.pattern_ty_map.get(local.pattern_id) else {
                            self.add_error(TypeDiagnostic::UnknownValue { expr: expr_id });
                            return None;
                        };
                        self.expr_ty_map.insert(expr_id.into(), pattern_ty.clone());
                    }
                    resolver::ResolveResult::Function(function) => {
                        self.expr_ty_map
                            .insert(expr_id.into(), Type::Function(function));
                    }
                    resolver::ResolveResult::Struct(_) => todo!(),
                };
            }
            ir::Expr::Lit(lit_kind) => {
                self.expr_ty_map
                    .insert(expr_id.into(), Type::Lit(*lit_kind));
            }
            ir::Expr::BlockExpr { stmts } => {
                let ty = self.block_expr(stmts);
                self.expr_ty_map.insert(expr_id.into(), ty);
            }
            ir::Expr::If {
                if_cond,
                if_branch,
                else_branch,
            } => {
                self.infer_expr(*if_cond);
                let if_branch_ty = self.block_expr(if_branch);
            }
            ir::Expr::Unary { expr, kind } => {}
            ir::Expr::Binary { left, right, kind } => {}
            ir::Expr::Return { expr } => {}
            ir::Expr::Index { base, index } => {}
            ir::Expr::Call { func, args } => {}
            ir::Expr::Paren { expr } => {}
            ir::Expr::Field { name, expr } => {}
            ir::Expr::Method { name, expr, args } => {}
            ir::Expr::Record { path, fields } => {}
        };
        Some(expr_id)
    }

    fn unify_nilable(&self, left: Type<'db>, right: Type<'db>) -> Type<'db> {
        match (left, right) {
            (Type::Lit(LitKind::Nil), not_nilable) | (not_nilable, Type::Lit(LitKind::Nil)) => {
                Type::Nilable(not_nilable.into())
            }
            (nilable @ Type::Nilable(_), _) | (_, nilable @ Type::Nilable(_)) => nilable,
            (left, right) => {
                assert!(right == left);
                right
            }
        }
    }

    fn block_expr(&mut self, stmts: &'db [Stmt<'db>]) -> Type<'db> {
        let mut ty = Type::Unit;
        for (position, stmt) in stmts.iter().with_position() {
            let output = self.infer_stmt(stmt);

            if matches!(position, Position::Only | Position::Last)
                && matches!(stmt, Stmt::Expr { semi: None, .. })
            {
                ty = output.unwrap_or(Type::Unit);
            }
        }
        ty
    }

    fn infer_stmt(&mut self, stmt: &'db Stmt) -> Option<Type<'db>> {
        match stmt {
            Stmt::Let {
                pattern: pattern_id,
                ty,
                expr: expr_id,
            } => {
                self.infer_expr(*expr_id);

                let infer_ty = self.expr_ty_map.get((*expr_id).into())?;
                if let Some(ty) = ty.clone().map(|ty| type_from_type_expr(self.db, ty)) {
                    if !self.can_assign_type(&ty, infer_ty) {
                        self.add_error(TypeDiagnostic::TypeMismatch {
                            expected: ty.clone(),
                            actual: infer_ty.clone(),
                            expr: *expr_id,
                        });
                    }
                    self.insert_pattern_ty(*pattern_id, ty);
                } else {
                    self.insert_pattern_ty(*pattern_id, infer_ty.clone());
                }
                Some(Type::Unit)
            }
            Stmt::Expr {
                expr: expr_id,
                semi,
            } => {
                self.infer_expr(*expr_id);
                Some(if semi.is_some() {
                    Type::Unit
                } else {
                    self.expr_ty_map.get((*expr_id).into())?.clone()
                })
            }
        }
    }

    fn can_assign_type(&self, lhs: &Type, rhs: &Type) -> bool {
        match (lhs, rhs) {
            (Type::Nilable(lhs), Type::Nilable(rhs)) => self.can_assign_type(lhs, rhs),
            (Type::Nilable(lhs), rhs) => {
                matches!(lhs.deref(), Type::Lit(LitKind::Nil)) || self.can_assign_type(lhs, rhs)
            }
            (rhs, Type::Nilable(_)) if !rhs.nilable() => false,
            (Type::Any, _) | (_, Type::Any) => true,
            (Type::Lit(lhs), Type::Lit(rhs)) => {
                rhs == lhs || (*lhs == LitKind::Float && *rhs == LitKind::Int)
            }
            (Type::Unit, Type::Unit) => true,
            (Type::Function(lhs), Type::Function(rhs)) => lhs == rhs,
            (Type::Struct(lhs), Type::Struct(rhs)) => lhs == rhs,
            (Type::BareFn(lhs), Type::Function(rhs)) => {
                let rhs_bare = func_to_bare(self.db, *rhs);
                lhs.params == rhs_bare.params && lhs.return_type == rhs_bare.return_type
            }
            (Type::BareFn(lhs), Type::BareFn(rhs)) => {
                lhs.params == rhs.params && lhs.return_type == rhs.return_type
            }
            _ => false,
        }
    }

    fn add_error(&mut self, diagnostic: TypeDiagnostic<'db>) {
        self.diagnostics.push(diagnostic);
    }
}

fn func_to_bare<'db>(db: &'db dyn salsa::Database, func: ir::Function<'db>) -> BareFn<'db> {
    BareFn {
        params: func
            .params(db)
            .iter()
            .map(|p| ty::Param {
                name: p.name(db),
                ty: type_from_type_expr(db, p.ty(db)),
            })
            .collect_vec(),
        return_type: func
            .output(db)
            .clone()
            .map(|o| type_from_type_expr(db, o))
            .unwrap_or_else(|| Type::Unit)
            .into(),
    }
}

fn type_from_type_expr<'db>(db: &'db dyn salsa::Database, ty: ir::TypeExpr<'db>) -> Type<'db> {
    match ty {
        ir::TypeExpr::Unknown => Type::Unknown,
        ir::TypeExpr::Nilable(nilable_type) => {
            Type::Nilable(Box::new(type_from_type_expr(db, *nilable_type)))
        }
        ir::TypeExpr::Lit(kind) => Type::Lit(kind),
        ir::TypeExpr::Any => Type::Any,
        ir::TypeExpr::Unit => Type::Unit,
        ir::TypeExpr::Struct(strct) => Type::Struct(strct),
        ir::TypeExpr::Function(function) => Type::Function(function),
        ir::TypeExpr::BareFunction { params, output } => Type::BareFn(BareFn {
            params: params
                .iter()
                .map(|p| ty::Param {
                    name: p.name(db),
                    ty: type_from_type_expr(db, p.ty(db)),
                })
                .collect_vec(),
            return_type: output
                .map(|o| type_from_type_expr(db, *o))
                .unwrap_or_else(|| Type::Unit)
                .into(),
        }),
    }
}

fn type_to_string<'db>(db: &'db dyn salsa::Database, ty: &'db Type) -> Ustr {
    match ty {
        Type::Unknown => "unknown".into(),
        Type::Unit => "()".into(),
        Type::Any => "any".into(),
        Type::Nilable(nilable) => Ustr::from(&format!("{}", type_to_string(db, nilable))),
        Type::Lit(lit_kind) => match lit_kind {
            LitKind::Float => "float".into(),
            LitKind::Int => "int".into(),
            LitKind::String => "string".into(),
            LitKind::Bool => "bool".into(),
            LitKind::Nil => "nil".into(),
        },
        Type::BareFn(bare_fn) => {
            let output = if matches!(*bare_fn.return_type, Type::Unit) {
                Cow::Borrowed("")
            } else {
                Cow::Owned(format!(" -> {}", type_to_string(db, &bare_fn.return_type)))
            };

            format!(
                "fn ({}){}",
                bare_fn
                    .params
                    .iter()
                    .map(|p| {
                        let name = if let Some(name) = p.name.as_ref() {
                            Cow::Owned(format!("{}: ", name))
                        } else {
                            Cow::Borrowed("")
                        };
                        format!("{}{}", name, type_to_string(db, &p.ty))
                    })
                    .join(", "),
                output
            )
            .into()
        }
        Type::Function(function) => {
            let output_ty = function
                .output(db)
                .clone()
                .map(|ty| type_from_type_expr(db, ty))
                .unwrap_or_else(|| Type::Unit);
            let output = if matches!(output_ty, Type::Unit) {
                Cow::Borrowed("")
            } else {
                Cow::Owned(format!(" -> {}", type_to_string(db, &output_ty)))
            };

            format!(
                "fn ({}){}",
                function
                    .params(db)
                    .iter()
                    .map(|p| {
                        //TODO: convert any patterns to strings
                        let name = if let Some(name) = p.name(db).as_ref() {
                            Cow::Owned(format!("{}: ", name))
                        } else {
                            Cow::Borrowed("")
                        };
                        format!(
                            "{}{}",
                            name,
                            type_to_string(db, &type_from_type_expr(db, p.ty(db)))
                        )
                    })
                    .join(", "),
                output
            )
            .into()
        }
        Type::Struct(_) => Ustr::from("todo: struct"),
        Type::Never => "!".into(),
    }
}

fn expr_node<'db>(
    db: &'db dyn salsa::Database,
    func: ir::Function<'db>,
    expr: ExprId,
) -> Option<SyntaxNode> {
    let source_map = ide::source_map(db, func);
    let parse = ide::parse(db, func.file(db));
    source_map
        .node_for_expr(expr)
        .map(|n| n.value.0.syntax_node_ptr().to_node(&parse.syntax_node(db)))
}

//TODO: move this to somewhere more appropriate
fn range_exlude_whitespace(node: SyntaxNode) -> TextRange {
    let range = node.text_range();

    if let Some(last_child) = node.last_child_or_token()
        && last_child.kind() == lexer::Syntax::WHITESPACE
    {
        TextRange::new(range.start(), range.end() - last_child.text_range().len())
    } else {
        range
    }
}

fn expr_range<'db>(
    db: &'db dyn salsa::Database,
    func: ir::Function<'db>,
    expr: ExprId,
) -> Option<TextRange> {
    expr_node(db, func, expr).map(range_exlude_whitespace)
}

fn expr_text<'db>(
    db: &'db dyn salsa::Database,
    func: ir::Function<'db>,
    expr: ExprId,
) -> Option<Ustr> {
    expr_node(db, func, expr).map(|n| n.text().to_string().into())
}

#[salsa::tracked]
pub fn type_diagnostics<'db>(
    db: &'db dyn salsa::Database,
    func: ir::Function<'db>,
) -> Vec<(String, TextRange)> {
    let body = ide::body(db, func);
    let result = infer_function(db, func);
    result
        .diagnostics
        .iter()
        .map(|d| match d {
            TypeDiagnostic::TypeMismatch {
                expected,
                actual,
                expr: expr_id,
            } => (
                format!(
                    "expected `{}`, got `{}`",
                    type_to_string(db, expected),
                    type_to_string(db, actual)
                ),
                {
                    let expr = body.expr(*expr_id);
                    let expr = if let Expr::BlockExpr { stmts } = expr {
                        stmts
                            .last()
                            .and_then(|e| match e {
                                Stmt::Let { .. } => None,
                                Stmt::Expr { expr, .. } => Some(*expr),
                            })
                            .unwrap_or(*expr_id)
                    } else {
                        *expr_id
                    };
                    expr_range(db, func, expr).unwrap()
                },
            ),
            TypeDiagnostic::UnknownValue { expr } => (
                format!(
                    "cannot find value `{}` in this scope",
                    expr_text(db, func, *expr).unwrap()
                ),
                expr_range(db, func, *expr).unwrap(),
            ),
        })
        .collect_vec()
}

#[salsa::tracked(returns(ref))]
pub fn infer_function<'db>(
    db: &'db dyn salsa::Database,
    func: ir::Function<'db>,
) -> InferenceResult<'db> {
    let body = ide::body(db, func);
    let scopes = scope::expr_scopes(db, func);
    let mut ctx = InferCtx {
        db,
        body,
        func,
        diagnostics: Default::default(),
        pattern_ty_map: Default::default(),
        expr_ty_map: Default::default(),
        scopes,
    };
    ctx.infer_function();
    ctx.results()
}
