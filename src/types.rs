use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    rc::Rc,
};

use itertools::{Itertools, Position};

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
    //TODO: remove implicit cast from any, add as operator with runtime type checking
    pub fn assignable_from(&self, other: &Type) -> bool {
        ((self.kind.eq(&other.kind)) && ((self.nilable, other.nilable) != (false, true))
            || (self.nilable && other.kind == TypeKind::Nil))
            || self.kind == TypeKind::Any
            || other.kind == TypeKind::Any
    }

    pub fn try_unwrap_block(self) -> Self {
        if matches!(self.kind, TypeKind::Block(_)) {
            if let TypeKind::Block(types) = self.kind.clone()
                && let Ok([item]) = TryInto::<[_; 1]>::try_into(types)
            {
                item
            } else {
                self
            }
        } else {
            self
        }
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

    pub fn non_nilable(kind: TypeKind) -> Self {
        Self {
            kind,
            nilable: false,
        }
    }

    pub fn nilable(kind: TypeKind) -> Self {
        Self {
            kind,
            nilable: true,
        }
    }

    pub fn make_nilable(self) -> Self {
        Self::nilable(self.kind)
    }

    pub fn make_non_nilable(self) -> Self {
        Self::non_nilable(self.kind)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FnParam {
    pub kind: common::FnParamKind,
    pub name: Option<Identifier>,
    pub ty: Type,
    pub default_value: Option<Type>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fn {
    pub params: Vec<FnParam>,
    pub returns: Vec<Type>,
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
    Fn(Fn),
    Any,
    // Never,
}

impl TypeKind {
    #[allow(clippy::should_implement_trait)]
    pub fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TypeKind::Fn(fn1), TypeKind::Fn(fn2)) => fn1
                .params
                .iter()
                .zip(fn2.params.iter())
                .zip(fn1.returns.iter())
                .zip(fn2.returns.iter())
                .all(|(((p1, p2), r1), r2)| p1.ty.eq(&p2.ty) && r1.eq(r2)),
            _ => (self == other) || (self.is_number() && other.is_number()),
        }
    }
    pub fn from_ident(ident: &str) -> Self {
        match ident {
            "int" => TypeKind::Int,
            "float" => TypeKind::Float,
            "nil" => TypeKind::Nil,
            "string" => TypeKind::String,
            "bool" => TypeKind::Bool,
            "any" => TypeKind::Any,
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
            TypeKind::Fn(func) => {
                let args = func.params.iter().map(|p| p.ty.to_string()).join(", ");
                let returns = func.returns.iter().map(|r| r.to_string()).join(", ");
                if returns.is_empty() {
                    write!(f, "fn({args})")
                } else {
                    write!(f, "fn({args}) -> {returns}")
                }
            }
            TypeKind::Any => write!(f, "any"),
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

//TODO: implements paths

#[derive(Debug, Default, Clone)]
pub struct Definitions {
    fns: HashMap<Identifier, Type>,
}

#[derive(Debug)]
struct FnContext {
    func: ast::Fn,
}

#[derive(Default)]
pub struct Context<'a> {
    defs: Definitions,
    scopes: Vec<Scope>,
    call_stack: Vec<FnContext>,
    source: &'a str,
    pub diagnostics: Vec<position::Diagnostic>,
}

impl<'a> Context<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            scopes: vec![Scope::new()],
            source,
            ..Default::default()
        }
    }

    fn push_call_stack(&mut self, context: FnContext) {
        self.call_stack.push(context);
    }

    fn pop_call_stack(&mut self) {
        self.call_stack.pop();
    }

    fn call_stack(&self) -> Option<&FnContext> {
        self.call_stack.last()
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

    fn ident_type(&mut self, ident: &Identifier, span: position::Span) -> Option<Type> {
        if let Some(ty) = self.scope().local(ident).cloned() {
            Some(ty)
        } else if let Some(ty) = self.defs.fns.get(ident).cloned() {
            Some(ty)
        } else {
            self.add_error(&format!("{} not found", ident), span);
            None
        }
    }

    fn add_error(&mut self, message: &str, span: position::Span) {
        self.diagnostics.push(position::Diagnostic {
            span,
            message: message.to_owned(),
        });
    }

    fn expr(&mut self, expr: &mut WithSpan<ast::Expr>) -> Option<Type> {
        Some(match &mut expr.value {
            ast::Expr::Nil => TypeKind::Nil.into(),
            ast::Expr::Number(number) => match number {
                ast::Number::Float(_) => TypeKind::Float.into(),
                ast::Number::Int(_) => TypeKind::Int.into(),
            },
            ast::Expr::Bool(_) => TypeKind::Bool.into(),
            ast::Expr::String(_, _) => TypeKind::String.into(),
            ast::Expr::Grouping(e) => self.expr(e)?,
            ast::Expr::Unary(unary) => {
                let expr_type = self.expr(&mut unary.expr)?;
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
                let left = self.expr(&mut binary_expr.left)?;
                let right = self.expr(&mut binary_expr.right)?;

                if !(left == right || (left.is_number() && right.is_number())) {
                    //FIXME: this will change with the introduction of vectors / operator
                    //overloading
                    self.add_error(
                        &format!(
                            "could not apply {} to {}: {} and {}: {}",
                            binary_expr.op.value,
                            self.source(binary_expr.left.span),
                            left,
                            self.source(binary_expr.right.span),
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
                                    self.source(binary_expr.left.span)
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
                                    self.source(binary_expr.left.span)
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
                                    self.source(binary_expr.left.span)
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
                let _cond_type = self.expr(&mut if_expr.condition)?;
                let then_type = self.block(&mut if_expr.then_branch.value.body)?;
                let else_type = if_expr.else_branch.as_mut().and_then(|b| self.expr(b));
                if else_type.is_none() && if_expr.else_branch.is_some() {
                    return None;
                }

                match else_type {
                    Some(else_type) => {
                        if !(else_type.kind.eq(&then_type.kind)
                            || (else_type.kind == TypeKind::Nil || then_type.kind == TypeKind::Nil))
                        {
                            self.add_error(
                                &format!(
                                    "if and else have incompatible types: {} and {}",
                                    else_type.kind, then_type.kind
                                ),
                                expr.span,
                            );
                            return None;
                        }

                        let ty_kind = match (&then_type.kind, &else_type.kind) {
                            (then_ty, else_ty)
                                if then_type.is_number() && else_type.is_number() =>
                            {
                                if then_type.kind == TypeKind::Float
                                    || else_type.kind == TypeKind::Float
                                {
                                    TypeKind::Float
                                } else {
                                    TypeKind::Int
                                }
                            }
                            (then_ty, TypeKind::Nil) => then_ty.clone(),
                            (TypeKind::Nil, else_ty) => else_ty.clone(),
                            (_, _) => else_type.kind.clone(),
                        };
                        Type {
                            kind: ty_kind,
                            nilable: then_type.nilable
                                || else_type.nilable
                                || then_type.kind == TypeKind::Nil
                                || else_type.kind == TypeKind::Nil,
                        }
                    }
                    None => then_type.make_nilable(),
                }
            }
            ast::Expr::Block(ast::Block { body, ty }) => {
                let block_ty = self.block(body)?;
                *ty = Some(block_ty.clone());
                block_ty
            }
            ast::Expr::Call(call) => self.call(WithSpan::new(call, expr.span))?,
        })
    }

    fn call(&mut self, call: WithSpan<&mut ast::Call>) -> Option<Type> {
        let callee_type = self.expr(&mut call.value.callee)?;
        call.value.callee_type = Some(callee_type.clone());
        match &callee_type.kind {
            TypeKind::Fn(func) => {
                let mut checked_params: Vec<Option<&FnParam>> = vec![None; func.params.len()];

                let mut ordered_amount = 0;
                for arg in call.value.args.iter_mut() {
                    let expr_ty = self.expr(&mut arg.expr)?;
                    let (i, param) =
                        if let Some(name) = &arg.name {
                            let Some((i, param)) = func.params.iter().enumerate().find(|(_, p)| {
                                p.name.as_ref().map(|n| n == name).unwrap_or_default()
                            }) else {
                                self.add_error(
                                    &format!("could not find parameter {name}"),
                                    arg.expr.span,
                                );
                                return None;
                            };
                            (i, param)
                        } else {
                            let Some(param) = func.params.get(ordered_amount) else {
                                //TODO: variadics
                                self.add_error("too many arguments", call.span);
                                return None;
                            };
                            (ordered_amount, param)
                        };
                    if checked_params[i].is_some() {
                        self.add_error(
                            &format!(
                                "attempt to pass parameter {} multiple times",
                                param.name.as_ref().unwrap()
                            ),
                            arg.expr.span,
                        );
                        return None;
                    }
                    if !param.ty.assignable_from(&expr_ty) {
                        self.add_error(
                            &format!(
                                "mismatched types: could not assign {} from {}",
                                param.ty, expr_ty
                            ),
                            arg.expr.span,
                        );
                        return None;
                    }

                    checked_params[i] = Some(param);
                    if arg.name.is_none() {
                        ordered_amount += 1;
                    }
                }

                if checked_params.len() < func.params.len() {
                    for (i, param) in func.params.iter().enumerate() {
                        if checked_params[i].is_some()
                            || param.ty.nilable
                            || param.default_value.is_some()
                        {
                            continue;
                        }
                        self.add_error("not enough arguments", call.span);
                        return None;
                    }
                }

                match func.returns.as_slice() {
                    [] => Some(TypeKind::Nil.into()),
                    [item] => Some(item.clone()),
                    types => Some(TypeKind::Block(types.to_vec()).into()),
                }
            }
            other => {
                self.add_error(&format!("expected function, got {}", other), call.span);
                None
            }
        }
    }

    fn block(&mut self, stmts: &mut [WithSpan<ast::Stmt>]) -> Option<Type> {
        self.push_scope();
        for stmt in stmts.iter_mut() {
            self.stmt(stmt);
        }

        let result = Some(match stmts.last_mut() {
            Some(last) => {
                if let ast::Stmt::Expr(ast::StmtExpr { exprs, semi }) = &mut last.value
                    && semi.is_none()
                {
                    let mut types = vec![];
                    for expr in exprs {
                        let ty = self.expr(expr)?;
                        types.push(ty);
                    }
                    TypeKind::Block(types).into()
                } else {
                    TypeKind::Block(vec![TypeKind::Nil.into()]).into()
                }
            }
            None => TypeKind::Block(vec![TypeKind::Nil.into()]).into(),
        })
        .map(|r: Type| r.try_unwrap_block());
        self.pop_scope();
        result
    }

    fn func(&mut self, func: WithSpan<&mut ast::Fn>) -> Option<()> {
        self.push_call_stack(FnContext {
            func: func.value.clone(),
        });
        self.push_scope();
        for (i, param) in func.value.params.iter_mut().enumerate() {
            self.scope_mut()
                .insert_local(&param.name.value, param.ty.value.clone());

            if let Some(default_value) = &mut param.default_value {
                let expr_ty = self.expr(default_value)?;
                if !param.ty.value.assignable_from(&expr_ty) {
                    self.add_error(
                        &format!(
                            "mismatched types: could not assign {} from {}",
                            param.ty.value, expr_ty,
                        ),
                        default_value.span,
                    );
                    return None;
                }
                let TypeKind::Fn(func) = &mut self.defs.fns.get_mut(&func.value.name).unwrap().kind
                else {
                    unreachable!()
                };
                func.params[i].default_value = Some(expr_ty);
            }
        }

        for (pos, stmt) in func.value.body.value.body.iter_mut().with_position() {
            match pos {
                Position::First | Position::Middle => {
                    self.stmt(stmt)?;
                }
                Position::Last | Position::Only => {
                    if let ast::Stmt::Expr(ast::StmtExpr { exprs, semi }) = &mut stmt.value
                        && semi.is_none()
                    {
                        self.rtrn(exprs, stmt.span)?;
                    } else {
                        self.stmt(stmt)?;
                    }
                }
            }
        }

        //TODO: ensure some return value is always present
        self.pop_scope();
        self.pop_call_stack();
        Some(())
    }

    fn item(&mut self, item: &mut WithSpan<ast::Item>) -> Option<()> {
        match &mut item.value {
            ast::Item::Fn(func) => self.func(WithSpan::new(func, item.span))?,
            //these are already checked when collecting definitions
            ast::Item::Extern(_) => (),
            ast::Item::Inline(_) => (),
        }
        Some(())
    }

    fn stmt(&mut self, stmt: &mut WithSpan<ast::Stmt>) -> Option<()> {
        match &mut stmt.value {
            ast::Stmt::Expr(stmt_expr) => {
                for expr in &mut stmt_expr.exprs {
                    self.expr(expr)?;
                }
            }
            ast::Stmt::Assign(assign) => {
                self.assign(assign, stmt.span)?;
            }
            ast::Stmt::Binding(binding) => {
                let mut value_types = vec![];
                for value in binding.values.iter_mut().flatten() {
                    value_types.append(&mut self.expr(value)?.flatten_block());
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
                                self.scope_mut().insert_local(&ident.value, value_ty);
                                return None;
                            }
                            let ty = parsed_ty
                                .clone()
                                .map(|t| t.value)
                                .unwrap_or_else(|| value_ty);
                            self.scope_mut().insert_local(&ident.value, ty);
                        }
                        None => self.scope_mut().insert_local(
                            &ident.value,
                            parsed_ty
                                .as_ref()
                                .map(|t| t.value.clone())
                                .unwrap_or_else(|| TypeKind::Nil.into()),
                        ),
                    }
                }
            }
            ast::Stmt::Empty => {}
            ast::Stmt::Return(exprs) => {
                self.rtrn(exprs, stmt.span)?;
            }
        };
        Some(())
    }

    fn rtrn(&mut self, exprs: &mut [WithSpan<ast::Expr>], span: position::Span) -> Option<()> {
        let mut values = vec![];
        for expr in exprs.iter_mut() {
            values.append(&mut self.expr(expr)?.flatten_block());
        }
        if let Some(returns) = self.call_stack().map(|stack| &stack.func.returns) {
            //functions with no return values are allowed to return nil
            if !(returns.is_empty()
                && matches!(
                    values.as_slice(),
                    [Type {
                        kind: TypeKind::Nil,
                        ..
                    }]
                ))
            {
                match values.len().cmp(&returns.len()) {
                    std::cmp::Ordering::Less => {
                        self.add_error("not enough return values provided", span);
                        return None;
                    }
                    std::cmp::Ordering::Greater => {
                        self.add_error("too much return values provided", span);
                        return None;
                    }
                    _ => {}
                }
            }
            for (value_ty, param_ty) in values.into_iter().zip(returns.iter().map(|r| &r.value)) {
                if !param_ty.assignable_from(&value_ty) {
                    self.add_error(
                        &format!(
                            "mismatched types: could not assign {} from {}",
                            param_ty, value_ty
                        ),
                        span,
                    );
                    return None;
                }
            }
        }
        Some(())
    }

    fn assign(&mut self, assign: &mut ast::Assign, span: position::Span) -> Option<()> {
        for (i, ident) in assign.idents.iter().enumerate() {
            let ident_ty = self.ident_type(&ident.value, ident.span)?;
            match assign.values.as_mut().and_then(|v| v.get_mut(i)) {
                Some(value) => {
                    let value_type = self.expr(value)?;
                    if !ident_ty.assignable_from(&value_type) {
                        self.add_error(
                            //TODO: improve errors messages. This should be something like "could
                            //not assign X from Y"
                            &format!(
                                "mismatched types: expected {}, got {}",
                                ident_ty, value_type
                            ),
                            span,
                        );
                        return None;
                    }
                }
                None => {
                    if !ident_ty.nilable {
                        self.add_error(
                            &format!("attempt to assign nil to a non-nilable type {}", ident_ty),
                            span,
                        );
                        return None;
                    }
                }
            }
        }
        Some(())
    }

    fn source(&self, range: position::Span) -> &str {
        let (start, end) = (range.start.0, range.end.0);
        if start == end {
            if start == 0 {
                &self.source[0..1]
            } else {
                &self.source[(range.start.0 - 1)..(range.end.0)]
            }
        } else {
            &self.source[(range.start.0)..(range.end.0)]
        }
    }

    fn add_inline_definition(&mut self, func: &ast::InlineFn) {
        self.defs.fns.insert(
            func.name.clone(),
            Type::non_nilable(TypeKind::Fn(Fn {
                params: func
                    .params
                    .iter()
                    .map(|p| FnParam {
                        kind: p.kind.clone(),
                        ty: p.ty.value.clone(),
                        name: Some(p.name.value.clone()),
                        default_value: None,
                    })
                    .collect(),
                returns: func.returns.iter().map(|r| r.value.clone()).collect(),
            })),
        );
    }

    fn add_extern_definition(&mut self, func: &ast::ExternFn) {
        self.defs.fns.insert(
            func.name.clone(),
            Type::non_nilable(TypeKind::Fn(Fn {
                params: func
                    .params
                    .iter()
                    .map(|p| FnParam {
                        kind: p.kind.clone(),
                        ty: p.ty.value.clone(),
                        name: Some(p.name.value.clone()),
                        default_value: None,
                    })
                    .collect(),
                returns: func.returns.iter().map(|r| r.value.clone()).collect(),
            })),
        );
    }

    fn add_func_definition(&mut self, func: &ast::Fn) {
        self.defs.fns.insert(
            func.name.clone(),
            Type::non_nilable(TypeKind::Fn(Fn {
                params: func
                    .params
                    .iter()
                    .map(|p| FnParam {
                        kind: p.kind.clone(),
                        ty: p.ty.value.clone(),
                        name: Some(p.name.value.clone()),
                        default_value: None,
                    })
                    .collect(),
                returns: func.returns.iter().map(|r| r.value.clone()).collect(),
            })),
        );
    }

    fn collect_definitions(&mut self, program: &[WithSpan<ast::Item>]) {
        for item in program {
            match &item.value {
                ast::Item::Fn(func) => self.add_func_definition(func),
                ast::Item::Inline(inline) => inline
                    .defs
                    .iter()
                    .for_each(|func| self.add_inline_definition(&func.value)),
                ast::Item::Extern(_extern) => {
                    _extern.defs.iter().for_each(|def| match &def.value {
                        ast::ExternDefinition::Fn(func) => self.add_extern_definition(func),
                    })
                }
            }
        }
    }

    pub fn type_check(&mut self, program: &mut [WithSpan<ast::Item>]) -> Option<()> {
        self.collect_definitions(program);
        for item in program {
            self.item(item);
        }
        Some(())
    }

    pub fn defs(&self) -> &Definitions {
        &self.defs
    }
}
