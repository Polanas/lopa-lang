use std::{
    borrow::Cow,
    collections::{HashMap, HashSet, linked_list},
    mem::transmute,
    ops::{Deref, Not},
};

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
        ir::{self, Expr, ExprId, PatternId, Stmt, Type},
        resolver::{self, ResolveResult},
        scope::{self, ScopeId},
    },
    ide::{self, diagnostics, source_map},
    parsing::{
        ast::{self, BinaryOpKind, SyntaxNode, SyntaxNodePtr},
        lexer,
    },
    ptr::Ptr,
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
    Expected {
        expected: Ustr,
        actual: Type<'db>,
        expr: ExprId,
    },
    UnsupportedBinaryOp {
        left: ExprId,
        right: ExprId,
        op: BinaryOpKind,
    },
    UnkownParamName {
        label: Ustr,
        expr: ExprId,
    },
    SameParamTwice {
        expr: ExprId,
        name: Ustr,
    },
    TooManyArguments {
        expr: ExprId,
        provided: usize,
        expected: usize,
    },
    TooFewArguments {
        expr: ExprId,
        provided: usize,
        expected: usize,
    },
    // UnknownType {
    //     ty: Type<'db>,
    //     expr: ExprId
    // },
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

    fn infer_function(&self) -> Option<()> {
        for (param_id, param) in self.body.params().iter().zip(self.func.params(self.db)) {
            self.insert_pattern_ty(*param_id, param.ty.clone());
        }
        let ty = self.infer_expr(self.body.body_expr(), None)?;
        if !self.can_assign_type(self.func.output(self.db), ty) {
            self.add_error(TypeDiagnostic::TypeMismatch {
                expected: self.func.output(self.db).clone(),
                actual: ty.clone(),
                expr: self.body.body_expr(),
            });
        }
        Some(())
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
    fn infer_expr(
        &self,
        expr_id: ExprId,
        expected: Option<&Type<'static>>,
    ) -> Option<&Type<'static>> {
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
                self.infer_expr(*if_cond, None);
                let if_branch_ty = self.infer_expr(*if_branch, None)?.clone().collapsed_nil();
                if let Some(else_branch) = else_branch {
                    let else_branch_ty =
                        self.infer_expr(*else_branch, None)?.clone().collapsed_nil();
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
                let lhs = self.infer_expr(*left, None)?;
                let rhs = self.infer_expr(*right, None)?;

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
                let ty = self.infer_expr(*expr, None)?.clone();
                self.insert_expr_ty(expr_id, ty);
            }
            ir::Expr::Call { func, args } => {
                let func = self.infer_expr(*func, None)?;
                let Type::Function(func) = func else {
                    self.add_error(TypeDiagnostic::Expected {
                        expected: "function".into(),
                        actual: func.clone(),
                        expr: expr_id,
                    });
                    return None;
                };
                let params_by_name = func.params_by_name(self.db);
                let params = func.params(self.db);

                let mut param_id = 0;
                let mut used_arg_names = vec![];
                let mut provided_count = 0;
                let mut provided_params = vec![];
                for arg in args {
                    match arg {
                        ir::Arg::Labeled { label, value } => {
                            provided_count += 1;
                            let Some(param) = params_by_name.get(label) else {
                                self.add_error(TypeDiagnostic::UnkownParamName {
                                    label: *label,
                                    expr: expr_id,
                                });
                                continue;
                            };
                            provided_params.push(param);
                            //TODO: pass the expected type here?
                            let arg_ty = self.infer_expr(*value, None)?;
                            if !self.can_assign_type(&param.ty, arg_ty) {
                                self.add_error(TypeDiagnostic::TypeMismatch {
                                    expected: param.ty.clone(),
                                    actual: arg_ty.clone(),
                                    expr: arg.value(),
                                });
                            }
                            used_arg_names.push(*label);
                        }
                        ir::Arg::NonLabeled { value } => {
                            param_id += 1;
                            provided_count += 1;
                            let Some(param) = params.get(param_id - 1) else {
                                continue;
                            };
                            provided_params.push(param);
                            if let Some(name) = param.name.as_ref()
                                && used_arg_names.contains(name)
                            {
                                self.add_error(TypeDiagnostic::SameParamTwice {
                                    expr: arg.value(),
                                    name: *name,
                                });
                                continue;
                            }
                            if let Some(name) = param.name.as_ref() {
                                used_arg_names.push(*name);
                            }

                            let arg_ty = self.infer_expr(*value, None)?;
                            if !self.can_assign_type(&param.ty, arg_ty) {
                                self.add_error(TypeDiagnostic::TypeMismatch {
                                    expected: param.ty.clone(),
                                    actual: arg_ty.clone(),
                                    expr: arg.value(),
                                });
                            }
                        }
                    }
                }
                for param in params {
                    if provided_params
                        .iter()
                        .filter_map(|p| p.name.map(|n| (n, p)))
                        .any(|(n, p)| n == *param.name.as_ref().unwrap())
                    {
                    } else {
                        if param.ty.is_nilable() {
                            provided_count += 1;
                        }
                    }
                }
                match provided_count.cmp(&params.len()) {
                    std::cmp::Ordering::Less => {
                        self.add_error(TypeDiagnostic::TooFewArguments {
                            expr: expr_id,
                            provided: provided_count,
                            expected: params.len(),
                        });
                    }
                    std::cmp::Ordering::Greater => {
                        self.add_error(TypeDiagnostic::TooManyArguments {
                            expr: expr_id,
                            provided: provided_count,
                            expected: params.len(),
                        });
                    }
                    _ => {}
                }
                self.insert_expr_ty(expr_id, func.output(self.db).clone());
            }
            ir::Expr::Return { expr } => {}
            ir::Expr::Index { base, index } => {}
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
                self.infer_expr(*expr_id, None);

                let infer_ty = self.expr_ty_map.get((*expr_id).into())?;
                if let Some(ty) = ty.clone() {
                    // if matches!(ty, Type::Unknown(_)) {
                    //     self.add_error(TypeDiagnostic::UnknownType { ty });
                    //     return None;
                    // }
                    if !self.can_assign_type(&ty, infer_ty) {
                        self.add_error(TypeDiagnostic::TypeMismatch {
                            expected: ty.clone(),
                            actual: infer_ty.clone(),
                            expr: *expr_id,
                        });
                        return None;
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
                self.infer_expr(*expr_id, None);
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
                matches!(rhs, Type::Lit(LitKind::Nil)) || self.can_assign_type(lhs, rhs)
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
                let rhs_bare = rhs.bare_fn_ty(self.db);
                lhs.params == rhs_bare.params && lhs.output == rhs_bare.output
            }
            (Type::BareFn(lhs), Type::BareFn(rhs)) => {
                lhs.params == rhs.params && lhs.output == rhs.output
            }
            _ => false,
        }
    }

    fn add_error(&self, diagnostic: TypeDiagnostic<'static>) {
        self.diagnostics.clone().push(diagnostic);
    }
}

fn type_to_string<'db>(db: &'db dyn salsa::Database, ty: &'db Type) -> Ustr {
    match ty {
        Type::Unknown(value) => Ustr::from(&format!("unknown type `{}`", value)),
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
            let output = if matches!(*bare_fn.output, Type::Unit) {
                Cow::Borrowed("")
            } else {
                Cow::Owned(format!(" -> {}", type_to_string(db, &bare_fn.output)))
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
            let output_ty = function.output(db).clone();
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
                    expr_range(db, func, expr).unwrap_or_default()
                },
            ),
            TypeDiagnostic::UnknownValue { expr } => (
                format!(
                    "cannot find value `{}` in this scope",
                    expr_text(db, func, *expr).unwrap_or_default()
                ),
                expr_range(db, func, *expr).unwrap_or_default(),
            ),
            TypeDiagnostic::UnsupportedBinaryOp { left, right, op } => (
                format!(
                    "`{}` operation not supported for `{}` and `{}`",
                    op,
                    expr_text(db, func, *left).unwrap_or_default(),
                    expr_text(db, func, *right).unwrap_or_default(),
                ),
                binary_op_range(db, func, *left).unwrap_or_default(),
            ),
            // TypeDiagnostic::UnknownType { ty } => (format),
            TypeDiagnostic::Expected {
                expected,
                actual,
                expr: expr_id,
            } => (
                format!(
                    "expected `{}`, got `{}`",
                    expected,
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
                    expr_range(db, func, expr).unwrap_or_default()
                },
            ),
            TypeDiagnostic::UnkownParamName { label, expr } => (
                format!("cannot find parameter with the name `{}`", label),
                expr_range(db, func, *expr).unwrap_or_default(),
            ),
            TypeDiagnostic::TooManyArguments {
                expr,
                expected,
                provided,
            } => (
                format!(
                    "too many arguments provided: expected {}, provided {}",
                    expected, provided
                ),
                expr_range(db, func, *expr).unwrap_or_default(),
            ),
            TypeDiagnostic::SameParamTwice { expr, name } => (
                format!("parameter `{}` is provided multiple times", name),
                expr_range(db, func, *expr).unwrap_or_default(),
            ),
            TypeDiagnostic::TooFewArguments {
                expr,
                provided,
                expected,
            } => (
                format!(
                    "too few arguments provided: expected {}, provided {}",
                    expected, provided
                ),
                expr_range(db, func, *expr).unwrap_or_default(),
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
