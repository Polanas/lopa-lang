use std::fmt::Display;

use crate::{
    ast, common,
    position::{self, WithSpan},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Nil,
    Bool,
    Int,
    Float,
    String,
}

impl Type {
    pub fn is_number(&self) -> bool {
        matches!(self, Type::Int | Type::Float)
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Nil => write!(f, "nil"),
            Type::Bool => write!(f, "bool"),
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::String => write!(f, "string"),
        }
    }
}

pub struct Context {
    pub diagnostics: Vec<position::Diagnostic>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            diagnostics: Default::default(),
        }
    }

    fn add_error(&mut self, message: &str, span: position::Span) {
        self.diagnostics.push(position::Diagnostic {
            span,
            message: message.to_owned(),
        });
    }

    fn expr(&mut self, expr: &mut WithSpan<ast::Expr>, source: &str) -> Option<Type> {
        Some(match &mut expr.value {
            ast::Expr::Nil => Type::Nil,
            ast::Expr::Number(number) => match number {
                ast::Number::Float(_) => Type::Float,
                ast::Number::Int(_) => Type::Int,
            },
            ast::Expr::Bool(_) => Type::Bool,
            ast::Expr::String(_) => Type::String,
            ast::Expr::Grouping(e) => self.expr(e, source)?,
            ast::Expr::Unary(unary) => {
                let expr_type = self.expr(&mut unary.expr, source)?;
                let unary_type = match unary.op.value {
                    common::UnaryOp::Not => Type::Bool,
                    common::UnaryOp::Negate => match expr_type {
                        ty @ (Type::Int | Type::Float) => ty,
                        ty => {
                            self.add_error(&format!("expected number, got {}", ty), expr.span);
                            return None;
                        }
                    },
                };
                if unary_type != expr_type {
                    self.add_error(
                        &format!(
                            "expected {}, got {}",
                            match unary.op.value {
                                common::UnaryOp::Not => "bool",
                                common::UnaryOp::Negate => "number",
                            },
                            expr_type
                        ),
                        expr.span,
                    );
                    return None;
                } else {
                    unary.ty = Some(unary_type);
                    unary_type
                }
            }
            ast::Expr::Binary(binary_expr) => {
                let left = self.expr(&mut binary_expr.left, source)?;
                let right = self.expr(&mut binary_expr.right, source)?;

                if left != right {
                    //TODO: this will change with the introduction of vectors / operator
                    //overloading
                    self.add_error(
                        &format!(
                            "expected {} and {} to be of the same type",
                            self.source(binary_expr.left.span, source),
                            self.source(binary_expr.right.span, source)
                        ),
                        expr.span,
                    );
                    return None;
                }

                let ty = match binary_expr.op.value {
                    common::BinaryOp::Div
                    | common::BinaryOp::Mult
                    | common::BinaryOp::Add
                    | common::BinaryOp::Sub
                    | common::BinaryOp::Modulo
                    | common::BinaryOp::Less
                    | common::BinaryOp::LessEqual
                    | common::BinaryOp::Greater
                    | common::BinaryOp::GreaterEqual => {
                        if !left.is_number() {
                            self.add_error(
                                &format!(
                                    "expected {} to be a number",
                                    self.source(binary_expr.left.span, source)
                                ),
                                expr.span,
                            );
                            return None;
                        }
                        match (left, right) {
                            (Type::Int, Type::Int) => Type::Int,
                            (Type::Float, Type::Float) => Type::Float,
                            _ => Type::Float,
                        }
                    }
                    common::BinaryOp::And | common::BinaryOp::Or => {
                        if left != Type::Bool {
                            self.add_error(
                                &format!(
                                    "expected {} to be a number",
                                    self.source(binary_expr.left.span, source)
                                ),
                                expr.span,
                            );
                            return None;
                        }
                        Type::Bool
                    }
                    _ => Type::Bool,
                };
                binary_expr.ty = Some(ty);
                ty
            }
            ast::Expr::Identifier(ident, ty) => {
                todo!()
            }
            ast::Expr::If(if_expr) => {
                todo!()
            }
            ast::Expr::Block(items, _) => {
                todo!()
            }
            ast::Expr::Call(_, items) => todo!(),
        })
    }
    fn block(&mut self, stmts: &mut [WithSpan<ast::Stmt>], source: &str) -> Option<()> {
        for stmt in stmts {
            self.stmt(stmt, source);
        }
        Some(())
    }

    fn stmt(&mut self, stmt: &mut WithSpan<ast::Stmt>, source: &str) -> Option<()> {
        todo!()
        // match &stmt.value {
        //     ast::Stmt::Expr(stmt_expr) => todo!(),
        //     ast::Stmt::Item(item) => todo!(),
        //     ast::Stmt::Assign(assign) => todo!(),
        //     ast::Stmt::Binding(binding) => todo!(),
        //     ast::Stmt::Print(_) => todo!(),
        //     ast::Stmt::Empty => todo!(),
        // };
        // Some(())
    }

    fn source<'a>(&self, range: position::Span, source: &'a str) -> &'a str {
        let (start, end) = (range.start.0, range.end.0);
        if start == end {
            if start == 0 {
                &source[0..1]
            } else {
                &source[(range.start.0 - 1)..(range.end.0)]
            }
        } else {
            &source[(range.start.0)..(range.end.0)]
        }
    }

    pub fn type_check(&mut self, program: &mut [WithSpan<ast::Stmt>], source: &str) -> Option<()> {
        for stmt in program {
            self.stmt(stmt, source);
        }
        Some(())
    }
}
