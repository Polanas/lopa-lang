use std::{
    borrow::Cow,
    collections::{HashMap, linked_list},
    mem::transmute,
    ops::{Deref, Not},
};

use super::Type;
use itertools::{Itertools, Position};
use la_arena::{ArenaMap, Idx, RawIdx};
use notify_rust::Notification;
use rowan::TextRange;
use ustr::Ustr;

use crate::{
    B,
    common::LitKind,
    def::{
        self, body,
        ir::{self, Expr, ExprId, PatternId, Stmt, TypeExpr},
        resolver::{self, ResolveResult},
        scope::{self, ScopeId},
    },
    ide::{self, diagnostics, source_map},
    parsing::{
        ast::{self, BinaryOpKind, SyntaxNode, SyntaxNodePtr},
        lexer,
    },
    ptr::Ptr,
    ty::{self, BareFn},
};

#[derive(Debug, Clone, PartialEq, Eq, salsa::Update)]
pub struct InferenceResult<'db> {
    pattern_ty_map: ArenaMap<PatternId, Type<'db>>,
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
    UnsupportedBinaryOp {
        left: ExprId,
        right: ExprId,
        op: BinaryOpKind,
    },
    UnknownValue {
        expr: ExprId,
    },
}

struct InferCtx {
    db: &'static dyn salsa::Database,
    body: &'static def::body::Body<'static>,
    func: ir::Function<'static>,
    scopes: &'static scope::ExprScopes<'static>,
    diagnostics: Ptr<Vec<TypeDiagnostic<'static>>>,
    pattern_ty_map: Ptr<ArenaMap<PatternId, Type<'static>>>,
    expr_ty_map: Ptr<ArenaMap<Idx<Expr<'static>>, Type<'static>>>,
}

impl InferCtx {
    fn results(self) -> InferenceResult<'static> {
        InferenceResult {
            pattern_ty_map: Ptr::try_unwrap(self.pattern_ty_map).unwrap(),
            expr_ty_map: Ptr::try_unwrap(self.expr_ty_map).unwrap(),
            diagnostics: Ptr::try_unwrap(self.diagnostics).unwrap(),
        }
    }

    fn infer_function(&self) {
        self.infer_expr(self.body.body_expr());
    }

    fn insert_pattern_ty(&self, pattern_id: PatternId, ty: Type<'static>) {
        self.pattern_ty_map.clone().insert(pattern_id, ty);
    }

    fn insert_expr_ty(&self, expr_id: ExprId, ty: Type<'static>) {
        self.expr_ty_map.clone().insert(expr_id.into(), ty);
    }

    fn pattern_ty(&self, pattern_id: PatternId) -> Option<&Type<'static>> {
        self.pattern_ty_map.get(pattern_id.into())
    }

    //TODO: add expected type
    fn infer_expr(&self, expr_id: ExprId) -> Option<&Type<'static>> {
        if let Some(ty) = self.expr_ty_map.get(expr_id.into()) {
            return Some(ty);
        };
        match self.body.expr(expr_id) {
            ir::Expr::Missing => return None,
            ir::Expr::Unit => {
                self.insert_expr_ty(expr_id, Type::Unit);
            }
            ir::Expr::Path(ustr) => {
                let result = resolver::resolve_name_for_expr(self.db, expr_id, self.func, &ustr[0]);
                let Some(result) = result else {
                    self.add_error(TypeDiagnostic::UnknownValue { expr: expr_id });
                    return None;
                };
                match result {
                    resolver::ResolveResult::Local(local) => {
                        let Some(pattern_ty) = self.pattern_ty(local.pattern_id) else {
                            self.add_error(TypeDiagnostic::UnknownValue { expr: expr_id });
                            return None;
                        };
                        self.insert_expr_ty(expr_id, pattern_ty.clone());
                    }
                    resolver::ResolveResult::Function(function) => {
                        self.insert_expr_ty(expr_id, Type::Function(function));
                    }
                    resolver::ResolveResult::Struct(_) => {}
                };
            }
            ir::Expr::Lit(lit_kind) => {
                self.insert_expr_ty(expr_id, Type::Lit(*lit_kind));
            }
            ir::Expr::BlockExpr { stmts } => {
                let ty = self.block_expr(stmts);
                self.insert_expr_ty(expr_id, ty);
            }
            ir::Expr::If {
                if_cond,
                if_branch,
                else_branch,
            } => {
                self.infer_expr(*if_cond);
                let if_branch_ty = self.infer_expr(*if_branch)?.clone().collapsed_nil();
                if let Some(else_branch) = else_branch {
                    let else_branch_ty = self.infer_expr(*else_branch)?.clone().collapsed_nil();
                    let Some(ty) = self.unify_nilable(if_branch_ty.clone(), else_branch_ty.clone())
                    else {
                        self.add_error(TypeDiagnostic::TypeMismatch {
                            expected: if_branch_ty,
                            actual: else_branch_ty,
                            expr: expr_id,
                        });
                        return None;
                    };

                    self.insert_expr_ty(expr_id, ty);
                } else {
                    self.insert_expr_ty(
                        expr_id,
                        self.unify_nilable(if_branch_ty, Type::Lit(LitKind::Nil))?,
                    );
                }
            }
            ir::Expr::Unary { expr, kind } => {}
            ir::Expr::Binary { left, right, kind } => {
                let lhs = self.infer_expr(*left)?;
                let rhs = self.infer_expr(*right)?;

                if let Some(result) = self.binary_op_result(lhs, rhs, *kind) {
                    self.insert_expr_ty(expr_id, result);
                } else {
                    self.add_error(TypeDiagnostic::UnsupportedBinaryOp {
                        left: *left,
                        right: *right,
                        op: *kind,
                    });
                }
            }
            ir::Expr::Paren { expr } => {
                let ty = self.infer_expr(*expr)?.clone();
                self.insert_expr_ty(expr_id, ty);
            }
            ir::Expr::Return { expr } => {}
            ir::Expr::Index { base, index } => {}
            ir::Expr::Call { func, args } => {}
            ir::Expr::Field { name, expr } => {}
            ir::Expr::Method { name, expr, args } => {}
            ir::Expr::Record { path, fields } => {}
            ir::Expr::SelfVar {} => {}
        };
        self.expr_ty_map.get(expr_id.into())
    }

    fn binary_op_result(
        &self,
        lhs: &Type<'static>,
        rhs: &Type<'static>,
        op: BinaryOpKind,
    ) -> Option<Type<'static>> {
        Some(match op {
            B![+] | B![*] | B![/] | B!["//"] | B!["//="] | B![%] => match (lhs, rhs) {
                (lhs, rhs) if lhs.is_int() && rhs.is_int() => Type::int(),
                (lhs, rhs) if lhs.is_number() && rhs.is_number() => Type::float(),
                _ => return None,
            },
            B![<] | B![<=] | B![>] | B![>=] => match (lhs, rhs) {
                (lhs, rhs) if lhs.is_number() && rhs.is_number() => Type::bool(),
                _ => return None,
            },
            B![==] | B![!=] => Type::bool(),
            B![or] => return self.unify_or(lhs.clone(), rhs.clone()),
            _ => return None,
        })
    }

    fn unify_or(&self, left: Type<'static>, right: Type<'static>) -> Option<Type<'static>> {
        Some(match (left, right) {
            (Type::Nilable(lhs), rhs) if rhs.is_nilable() => *lhs,
            (lhs, rhs) if !lhs.is_nilable() && rhs.is_nilable() => lhs,
            (lhs, Type::Nilable(rhs)) if lhs.is_nilable() => *rhs,
            (lhs, rhs) if lhs.is_nilable() && !rhs.is_nilable() => rhs,
            (left, right) if left == right => right,
            _ => return None,
        })
    }

    fn unify_nilable(&self, left: Type<'static>, right: Type<'static>) -> Option<Type<'static>> {
        Some(match (left, right) {
            (lhs, rhs) if !lhs.is_nilable() && rhs.is_nilable() => Type::Nilable(lhs.into()),
            (lhs, rhs) if lhs.is_nilable() && !rhs.is_nilable() => Type::Nilable(rhs.into()),
            (left, right) if left == right => right,
            _ => return None,
        })
    }

    fn block_expr(&self, stmts: &'static [Stmt<'static>]) -> Type<'static> {
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

    fn infer_stmt(&self, stmt: &'static Stmt) -> Option<Type<'static>> {
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
                matches!(rhs.deref(), Type::Lit(LitKind::Nil)) || self.can_assign_type(lhs, rhs)
            }
            (rhs, Type::Nilable(_)) if !rhs.is_nilable() => false,
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

    fn add_error(&self, diagnostic: TypeDiagnostic<'static>) {
        self.diagnostics.clone().push(diagnostic);
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
        Type::Nilable(nilable) => Ustr::from(&format!("{}?", type_to_string(db, nilable))),
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

fn expr_range<'db>(
    db: &'db dyn salsa::Database,
    func: ir::Function<'db>,
    expr: ExprId,
) -> Option<TextRange> {
    expr_node(db, func, expr).map(|node| node.text_range())
}

fn binary_op_range<'db>(
    db: &'db dyn salsa::Database,
    func: ir::Function<'db>,
    expr: ExprId,
) -> Option<TextRange> {
    let node = expr_node(db, func, expr)?;
    ast::BinaryExpr(node.parent()?)
        .op_token()
        .map(|t| t.text_range())
}

fn expr_text<'db>(
    db: &'db dyn salsa::Database,
    func: ir::Function<'db>,
    expr: ExprId,
) -> Option<Ustr> {
    let range = expr_range(db, func, expr)?;
    let contents = func.file(db).contents(db).read().unwrap();
    let contents = contents.as_str();
    Some(Ustr::from(&contents[range]))
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
            TypeDiagnostic::UnsupportedBinaryOp { left, right, op } => (
                format!(
                    "`{}` operation not supported for `{}` and `{}`",
                    op,
                    expr_text(db, func, *left).unwrap(),
                    expr_text(db, func, *right).unwrap(),
                ),
                binary_op_range(db, func, *left).unwrap(),
            ),
        })
        .collect_vec()
}

#[salsa::tracked(returns(ref))]
pub fn infer_function<'db>(
    db: &'db dyn salsa::Database,
    func: ir::Function<'db>,
) -> InferenceResult<'db> {
    let db: &'static dyn salsa::Database = unsafe { transmute(db) };
    let func: ir::Function<'static> = unsafe { transmute(func) };

    let body = ide::body(db, func);
    let scopes = scope::expr_scopes(db, func);
    let mut ctx = InferCtx {
        db,
        body,
        func,
        diagnostics: Ptr::new(Default::default()),
        pattern_ty_map: Ptr::new(Default::default()),
        expr_ty_map: Ptr::new(Default::default()),
        scopes,
    };
    ctx.infer_function();
    ctx.results()
}
