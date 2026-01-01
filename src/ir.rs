use std::collections::{HashMap, HashSet};

use crate::{ast, common::*, luajit, position::WithSpan};

pub type Identifier = String;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ConstantId(pub u16);

#[derive(Clone, Copy, Debug)]
pub enum Table {
    Constant(ConstantId),
    Empty,
}

#[derive(Clone, Debug)]
pub enum Value {
    Int16(i16),
    Number(ConstantId),
    String(ConstantId),
    Global(ConstantId),
    Bool(bool),
    Nil,
    Table(Table),
    Local(String),
}

#[derive(Clone, Copy, Debug)]
pub enum Primitive {
    Nil,
    False,
    True,
}

#[derive(Clone, Copy, Debug)]
pub enum ConditionalJump {
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Equal,
    NotEqual,
    EqualString(ConstantId),
    NotEqualString(ConstantId),
    EqualNumer(ConstantId),
    NotEqualNumer(ConstantId),
    EqualPrimitive(Primitive),
    NotEqualPrimitive(Primitive),
}

#[derive(Clone, Debug)]
pub enum Instruction {
    Push(Value),
    Pop,
    Binding(BindingKind, Vec<Identifier>),
    Assign(Vec<Identifier>),
    Unary(UnaryOp),
    Global(Identifier),
    Binary(BinaryOp),
    Jump(ConditionalJump, usize),
    ScopeStart,
    ScopeEnd,
    StmtEnd,
    Print,
}

pub struct Prototype {}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum NumberConstant {
    Int(u32),
    Float(ordered_float::OrderedFloat<f64>),
}

#[derive(Default, Debug)]
pub struct FunctionContext {
    number_constants: HashMap<NumberConstant, ConstantId>,
    pushes_amount: Vec<usize>,
    pub string_constants: HashMap<String, ConstantId>,
    pub gc_constants: Vec<luajit::GCConstant>,
    pub num_constants: Vec<luajit::NumberConstant>,
    pub num_params: usize,
    pub children: Vec<FunctionContext>,
    pub instructions: Vec<Instruction>,
    locals: HashSet<String>,
}

impl FunctionContext {
    fn push(&mut self, value: Value) {
        if let Some(amount) = self.pushes_amount_mut() {
            *amount += 1;
        }
        self.instructions.push(Instruction::Push(value));
    }

    fn pushes_amount(&self) -> Option<usize> {
        self.pushes_amount.last().cloned()
    }

    fn pushes_amount_mut(&mut self) -> Option<&mut usize> {
        self.pushes_amount.last_mut()
    }

    fn push_pushes_amount(&mut self) {
        self.pushes_amount.push(0);
    }

    fn pop_pushes_amount(&mut self) -> Option<usize> {
        self.pushes_amount.pop()
    }

    fn pop(&mut self) {
        self.instructions.push(Instruction::Pop);
    }

    fn stmt_end(&mut self) {
        self.instructions.push(Instruction::StmtEnd);
    }

    fn binding(&mut self, idents: Vec<Identifier>, kind: BindingKind) {
        for ident in idents.iter() {
            match kind {
                BindingKind::Local => {
                    self.locals.insert(ident.clone());
                }
                BindingKind::Global => {
                    self.const_string(ident);
                }
            };
        }
        self.instructions.push(Instruction::Binding(kind, idents));
    }

    fn assign(&mut self, idents: Vec<Identifier>) {
        self.instructions.push(Instruction::Assign(idents));
    }

    fn upary(&mut self, op: UnaryOp) {
        self.instructions.push(Instruction::Unary(op));
    }

    fn binary(&mut self, op: BinaryOp) {
        self.instructions.push(Instruction::Binary(op));
    }

    fn jump(&mut self, jump: ConditionalJump, amount: usize) {
        self.instructions.push(Instruction::Jump(jump, amount));
    }

    fn scope_start(&mut self) {
        self.instructions.push(Instruction::ScopeStart);
    }

    fn scope_end(&mut self) {
        self.instructions.push(Instruction::ScopeEnd);
    }

    fn expr(&mut self, expr: &ast::Expr) {
        match expr {
            ast::Expr::Nil => self.push(Value::Nil),
            ast::Expr::Number(number) => match number {
                ast::Number::Float(f) => {
                    let id = self
                        .const_number(NumberConstant::Float(ordered_float::OrderedFloat::from(*f)));
                    self.push(Value::Number(id));
                }
                ast::Number::Int(i) => {
                    if let Ok(i) = i16::try_from(*i) {
                        self.push(Value::Int16(i));
                    } else if let Ok(i) = u32::try_from(*i) {
                        let id = self.const_number(NumberConstant::Int(i));
                        self.push(Value::Number(id));
                    } else {
                        let id = self.const_number(NumberConstant::Float(
                            ordered_float::OrderedFloat::from(*i as f64),
                        ));
                        self.push(Value::Number(id));
                    }
                }
            },
            ast::Expr::Bool(b) => self.push(Value::Bool(*b)),
            ast::Expr::String(s) => {
                let id = self.const_string(s);
                self.push(Value::String(id));
            }
            ast::Expr::Grouping(e) => {
                self.expr(&e.value);
            }
            ast::Expr::Unary(op, expr) => {
                self.expr(&expr.value);
                self.upary(op.value);
            }
            ast::Expr::Binary(binary_expr) => {
                self.expr(&binary_expr.left.value);
                self.expr(&binary_expr.right.value);
                self.binary(binary_expr.op.value);
            }
            ast::Expr::Identifier(i) => {
                if self.locals.contains(i) {
                    self.push(Value::Local(i.clone()));
                } else {
                    let str = self.const_string(i);
                    self.push(Value::Global(str));
                }
            }
            ast::Expr::Call(_, items) => todo!(),
            ast::Expr::If(if_expr) => todo!(),
            ast::Expr::Block(stmts) => self.block(&stmts),
        }
    }

    fn record_pushes(&mut self, func: impl FnOnce(&mut Self)) -> usize {
        self.push_pushes_amount();
        func(self);
        self.pushes_amount.pop().unwrap()
    }

    fn block(&mut self, stmts: &[WithSpan<ast::Stmt>]) {
        self.scope_start();
        match &stmts {
            [] => {
                self.scope_end();
                self.push(Value::Nil);
            }
            [stmts @ .., last] => {
                for stmt in stmts {
                    if matches!(
                        stmt.value,
                        ast::Stmt::Expr(ast::StmtExpr { semi: None, .. })
                    ) {
                        panic!("statement expression not allowed");
                    }
                    self.stmt(&stmt.value);
                }

                if let ast::Stmt::Expr(ast::StmtExpr { semi, .. }) = &last.value {
                    if semi.is_some() {
                        self.stmt(&last.value);
                        self.scope_end();
                        self.push(Value::Nil);
                    } else {
                        self.scope_end();
                        self.stmt(&last.value);
                    }
                } else {
                    self.stmt(&last.value);
                    self.scope_end();
                    self.push(Value::Nil);
                }

                // if !matches!(&last.value, ast::Stmt::Expr(_)) {
                //     self.push(Value::Nil);
                // }
            }
        }
    }

    fn const_number(&mut self, number: NumberConstant) -> ConstantId {
        if let Some(id) = self.number_constants.get(&number) {
            *id
        } else {
            let id = ConstantId(self.num_constants.len() as u16);
            self.number_constants.insert(number, id);
            self.num_constants.push(match number {
                NumberConstant::Int(i) => luajit::NumberConstant::Int(i),
                NumberConstant::Float(f) => luajit::NumberConstant::Num(f.into()),
            });
            id
        }
    }

    fn const_string(&mut self, string: &str) -> ConstantId {
        if let Some(id) = self.string_constants.get(string) {
            *id
        } else {
            let id = ConstantId(self.gc_constants.len() as u16);
            self.string_constants.insert(string.to_owned(), id);
            self.gc_constants
                .push(luajit::GCConstant::Str(string.to_owned()));
            id
        }
    }

    fn stmt(&mut self, stmt: &ast::Stmt) {
        match &stmt {
            ast::Stmt::Expr(exprs) => {
                for expr in &exprs.exprs {
                    //TODO: account for when statement expr is not the last one (also account for
                    //comma)
                    self.expr(&expr.value);
                }
            }
            ast::Stmt::Item(item) => {}
            ast::Stmt::Binding(binding) => {
                let pushes = self.record_pushes(|_self| {
                    if let Some(exprs) = &binding.values {
                        for expr in exprs {
                            _self.expr(&expr.value);
                        }
                    } else {
                        for _ in binding.identifiers.iter() {
                            _self.push(Value::Nil);
                        }
                    }
                });
                for _ in pushes..binding.identifiers.len() {
                    self.push(Value::Nil);
                }

                self.binding(
                    binding
                        .identifiers
                        .iter()
                        .map(|i| i.value.clone())
                        .collect(),
                    binding.kind,
                );
            }
            ast::Stmt::Print(v) => {
                self.expr(&v.value);
                self.instructions.push(Instruction::Print);
            }
            ast::Stmt::Empty => (),
            ast::Stmt::Assign(idents, values) => {
                for ident in idents.iter() {
                    if !self.locals.contains(&ident.value) {
                        self.const_string(&ident.value);
                    }
                }
                let pushes = self.record_pushes(|_self| {
                    for value in values {
                        _self.expr(&value.value);
                    }
                });
                for _ in pushes..idents.len() {
                    self.push(Value::Nil);
                }

                self.assign(idents.iter().map(|i| i.value.clone()).collect());
            }
        }
        self.stmt_end();
    }

    fn generate(&mut self, ast: &[WithSpan<ast::Stmt>]) {
        self.scope_start();
        for stmt in ast {
            self.stmt(&stmt.value);
        }
        self.scope_end();
    }
}

pub fn generate(program: &[WithSpan<ast::Stmt>]) -> FunctionContext {
    let mut context = FunctionContext::default();
    context.generate(program);
    context
}
