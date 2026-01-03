use std::{collections::HashMap, fmt::Display};

use crate::{
    ast::{self},
    common::{self, Identifier},
    position::{self, WithSpan},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Type {
    pub kind: TypeKind,
    pub nilable: bool,
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.kind, if self.nilable { "?" } else { "" })
    }
}

impl Type {
    pub fn assignable_from(&self, other: &Type) -> bool {
        (other.kind == self.kind || (self.is_number() && other.is_number()))
            && ((self.nilable, other.nilable) != (false, true))
            || (self.nilable && other.kind == TypeKind::Nil)
    }

    pub fn flatten_block(&self) -> Vec<Type> {
        fn flatten_inner(ty: &Type, types: &mut Vec<Type>) {
            if let TypeKind::Block(block) = &ty.kind {
                block.iter().for_each(|b| {
                    flatten_inner(b, types);
                });
            } else {
                types.push(ty.clone());
            }
        }

        let mut types = vec![];
        flatten_inner(self, &mut types);
        types
    }

    pub fn is_number(&self) -> bool {
        matches!(self.kind, TypeKind::Int | TypeKind::Float) && !self.nilable
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeKind {
    Nil,
    Bool,
    Int,
    Float,
    String,
    Custom,
    Block(Vec<Type>),
}

impl TypeKind {
    pub fn from_ident(ident: &str) -> Self {
        match ident {
            "int" => TypeKind::Int,
            "float" => TypeKind::Float,
            "nil" => TypeKind::Nil,
            "string" => TypeKind::String,
            "bool" => TypeKind::Bool,
            _ => TypeKind::Custom,
        }
    }
}

impl From<TypeKind> for Type {
    fn from(kind: TypeKind) -> Self {
        Type {
            kind,
            nilable: false,
        }
    }
}

impl TypeKind {
    pub fn is_number(&self) -> bool {
        matches!(self, TypeKind::Int | TypeKind::Float)
    }
}

impl Display for TypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeKind::Nil => write!(f, "nil"),
            TypeKind::Bool => write!(f, "bool"),
            TypeKind::Int => write!(f, "int"),
            TypeKind::Float => write!(f, "float"),
            TypeKind::String => write!(f, "string"),
            TypeKind::Custom => write!(f, "custom"),
            TypeKind::Block(_) => write!(f, "block"),
        }
    }
}

#[derive(Clone, Debug)]
struct Scope {
    locals: HashMap<Identifier, Type>,
}

impl Scope {
    fn new() -> Self {
        Self {
            locals: Default::default(),
        }
    }

    fn insert_local(&mut self, name: &str, ty: Type) {
        self.locals.insert(name.to_owned(), ty);
    }

    fn local(&self, name: &str) -> Option<&Type> {
        self.locals.get(name)
    }
}

#[derive(Default)]
pub struct Context {
    globals: HashMap<Identifier, Type>,
    scopes: Vec<Scope>,
    pub diagnostics: Vec<position::Diagnostic>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::new()],
            ..Default::default()
        }
    }

    fn push_scope(&mut self) {
        let scope = self.scope().clone();
        self.scopes.push(scope);
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn scope(&self) -> &Scope {
        &self.scopes[self.scopes.len() - 1]
    }

    fn scope_mut(&mut self) -> &mut Scope {
        let len = self.scopes.len();
        &mut self.scopes[len - 1]
    }

    fn insert_global(&mut self, name: &str, ty: Type) {
        self.globals.insert(name.to_owned(), ty);
    }

    fn insert_ident(&mut self, name: &str, ty: Type, kind: common::BindingKind) {
        match kind {
            common::BindingKind::Local => self.scope_mut().insert_local(name, ty),
            common::BindingKind::Global => self.insert_global(name, ty),
        }
    }

    fn ident_type(&mut self, ident: &Identifier, span: position::Span) -> Option<Type> {
        let ty = self
            .scope()
            .local(ident)
            .or_else(|| self.globals.get(ident))
            .cloned();
        if ty.is_none() {
            self.add_error(&format!("{} not found", ident), span);
        }
        ty
    }

    fn add_error(&mut self, message: &str, span: position::Span) {
        self.diagnostics.push(position::Diagnostic {
            span,
            message: message.to_owned(),
        });
    }

    fn expr(&mut self, expr: &mut WithSpan<ast::Expr>, source: &str) -> Option<Type> {
        Some(match &mut expr.value {
            ast::Expr::Nil => TypeKind::Nil.into(),
            ast::Expr::Number(number) => match number {
                ast::Number::Float(_) => TypeKind::Float.into(),
                ast::Number::Int(_) => TypeKind::Int.into(),
            },
            ast::Expr::Bool(_) => TypeKind::Bool.into(),
            ast::Expr::String(_) => TypeKind::String.into(),
            ast::Expr::Grouping(e) => self.expr(e, source)?,
            ast::Expr::Unary(unary) => {
                let expr_type = self.expr(&mut unary.expr, source)?;
                let unary_type = match &unary.op.value {
                    common::UnaryOp::Not => TypeKind::Bool,
                    common::UnaryOp::Negate => match &expr_type.kind {
                        ty @ (TypeKind::Int | TypeKind::Float) => ty.clone(),
                        ty => {
                            self.add_error(&format!("expected number, got {}", ty), expr.span);
                            return None;
                        }
                    },
                };
                if unary_type != expr_type.kind {
                    self.add_error(
                        &format!(
                            "expected {}, got {}",
                            match unary.op.value {
                                common::UnaryOp::Not => "bool",
                                common::UnaryOp::Negate => "number",
                            },
                            expr_type.kind
                        ),
                        expr.span,
                    );
                    return None;
                } else {
                    unary.ty = Some(unary_type.clone().into());
                    unary_type.into()
                }
            }
            ast::Expr::Binary(binary_expr) => {
                let left = self.expr(&mut binary_expr.left, source)?;
                let right = self.expr(&mut binary_expr.right, source)?;

                if !(left == right || (left.is_number() && right.is_number())) {
                    //TODO: this will change with the introduction of vectors / operator
                    //overloading
                    self.add_error(
                        &format!(
                            "could not apply {} to {}: {} and {}: {}",
                            binary_expr.op.value,
                            self.source(binary_expr.left.span, source),
                            left,
                            self.source(binary_expr.right.span, source),
                            right
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
                    | common::BinaryOp::Modulo => {
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
                        match (&left.kind, &right.kind) {
                            (TypeKind::Int, TypeKind::Int) => TypeKind::Int,
                            (TypeKind::Float, TypeKind::Float) => TypeKind::Float,
                            _ => TypeKind::Float,
                        }
                    }
                    common::BinaryOp::Less
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
                        TypeKind::Bool
                    }
                    common::BinaryOp::And | common::BinaryOp::Or => {
                        if left.kind != TypeKind::Bool {
                            self.add_error(
                                &format!(
                                    "expected {} to be a number",
                                    self.source(binary_expr.left.span, source)
                                ),
                                expr.span,
                            );
                            return None;
                        }
                        TypeKind::Bool
                    }
                    _ => TypeKind::Bool,
                };
                binary_expr.types = Some((left, right, ty.clone().into()));
                ty.into()
            }
            ast::Expr::Identifier(ident, ty) => {
                let ident_type = self.ident_type(ident, expr.span)?;
                *ty = Some(ident_type.clone());
                ident_type
            }
            ast::Expr::If(if_expr) => {
                let _cond_type = self.expr(&mut if_expr.condition, source)?;
                let then_type = self.block(&mut if_expr.then_branch.value.body, source)?;
                let else_type = if_expr
                    .else_branch
                    .as_mut()
                    .and_then(|b| self.expr(b, source));
                if else_type.is_none() && if_expr.else_branch.is_some() {
                    return None;
                }

                match else_type {
                    Some(else_type) => {
                        if else_type != then_type {
                            self.add_error("if and else have incompatible types", expr.span);
                            return None;
                        }

                        else_type
                    }
                    None => then_type,
                }
            }
            ast::Expr::Block(ast::Block { body, ty }) => {
                let block_ty = self.block(body, source)?;
                *ty = Some(block_ty.clone());
                block_ty
            }
            ast::Expr::Call(_, items) => todo!(),
        })
    }

    fn block(&mut self, stmts: &mut [WithSpan<ast::Stmt>], source: &str) -> Option<Type> {
        self.push_scope();
        for stmt in stmts.iter_mut() {
            self.stmt(stmt, source);
        }

        let result = Some(match stmts.last_mut() {
            Some(last) => {
                if let ast::Stmt::Expr(ast::StmtExpr { exprs, semi }) = &mut last.value
                    && semi.is_none()
                {
                    let mut types = vec![];
                    for expr in exprs {
                        let ty = self.expr(expr, source)?;
                        types.push(ty);
                    }
                    TypeKind::Block(types).into()
                } else {
                    TypeKind::Block(vec![TypeKind::Nil.into()]).into()
                }
            }
            None => TypeKind::Block(vec![TypeKind::Nil.into()]).into(),
        });
        self.pop_scope();
        result
    }

    fn stmt(&mut self, stmt: &mut WithSpan<ast::Stmt>, source: &str) -> Option<()> {
        match &mut stmt.value {
            ast::Stmt::Expr(stmt_expr) => {
                for expr in &mut stmt_expr.exprs {
                    self.expr(expr, source)?;
                }
            }
            ast::Stmt::Item(item) => {}
            ast::Stmt::Assign(assign) => {
                for (i, ident) in assign.idents.iter().enumerate() {
                    let ident_ty = self.ident_type(&ident.value, ident.span)?;
                    match assign.values.as_mut().and_then(|v| v.get_mut(i)) {
                        Some(value) => {
                            let value_type = self.expr(value, source)?;
                            if !ident_ty.assignable_from(&value_type) {
                                self.add_error(
                                    &format!(
                                        "mismatched types: expected {}, got {}",
                                        ident_ty, value_type
                                    ),
                                    stmt.span,
                                );
                                return None;
                            }
                        }
                        None => {
                            if !ident_ty.nilable {
                                self.add_error(
                                    &format!(
                                        "attempt to assign nil to a non-nilable type {}",
                                        ident_ty
                                    ),
                                    stmt.span,
                                );
                                return None;
                            }
                        }
                    }
                }
            }
            ast::Stmt::Binding(binding) => {
                let mut value_types = vec![];
                for value in binding.values.iter_mut().flatten() {
                    value_types.append(&mut self.expr(value, source)?.flatten_block());
                }
                for (i, ident) in binding.idents.iter().enumerate() {
                    let parsed_ty = &binding.types[i];
                    let value_ty = value_types
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| TypeKind::Nil.into());

                    match binding.values {
                        Some(_) => {
                            if let Some(parsed_ty) = parsed_ty
                                && !parsed_ty.value.assignable_from(&value_ty)
                            {
                                self.add_error(
                                    &format!(
                                        "mismatched types: expected {}, got {}",
                                        parsed_ty.value, &value_ty
                                    ),
                                    stmt.span,
                                );
                                self.insert_ident(&ident.value, value_ty, binding.kind);
                                return None;
                            }
                            let ty = parsed_ty
                                .clone()
                                .map(|t| t.value)
                                .unwrap_or_else(|| value_ty);
                            self.insert_ident(&ident.value, ty, binding.kind);
                        }
                        None => self.insert_ident(
                            &ident.value,
                            parsed_ty
                                .as_ref()
                                .map(|t| t.value.clone())
                                .unwrap_or_else(|| TypeKind::Nil.into()),
                            binding.kind,
                        ),
                    }
                }
            }
            ast::Stmt::Print(_) => {}
            ast::Stmt::Empty => todo!(),
            ast::Stmt::Return(items) => todo!(),
        };
        Some(())
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

    fn init_globals(&mut self, program: &[WithSpan<ast::Stmt>]) {
        for stmt in program {
            match &stmt.value {
                ast::Stmt::Binding(binding) if binding.kind == common::BindingKind::Global => {
                    for ident in &binding.idents {
                        self.insert_global(&ident.value, TypeKind::Nil.into());
                    }
                }
                ast::Stmt::Item(_) => todo!(),
                _ => {}
            }
        }
    }

    pub fn type_check(&mut self, program: &mut [WithSpan<ast::Stmt>], source: &str) -> Option<()> {
        self.init_globals(program);
        for stmt in program {
            self.stmt(stmt, source);
        }
        Some(())
    }
}
