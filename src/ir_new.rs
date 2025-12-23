use std::collections::HashMap;

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
    Bool(bool),
    Nil,
    Table(Table),
    Identifier(String),
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

#[derive(Clone, Copy, Debug)]
pub enum Global {
    Get,
    Set,
}

#[derive(Clone, Debug)]
pub enum Instruction {
    Push(Value),
    Pop,
    Local(Identifier),
    Global(Global),
    Assign(Identifier),
    Unary(UnaryOp),
    Binary(BinaryOp),
    Jump(ConditionalJump, usize),
    StmtEnd,
    ScopeStart,
    ScopeEnd,
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
    string_constants: HashMap<String, ConstantId>,
    number_constants: HashMap<NumberConstant, ConstantId>,
    pub gc_constants: Vec<luajit::GCConstant>,
    pub num_constants: Vec<luajit::NumberConstant>,
    pub num_params: usize,
    pub children: Vec<FunctionContext>,
    pub instructions: Vec<Instruction>,
}

impl FunctionContext {
    fn push(&mut self, value: Value) {
        self.instructions.push(Instruction::Push(value));
    }

    fn pop(&mut self) {
        self.instructions.push(Instruction::Pop);
    }

    fn local(&mut self, ident: Identifier) {
        self.instructions.push(Instruction::Local(ident))
    }

    fn assign(&mut self, ident: Identifier) {
        self.instructions.push(Instruction::Assign(ident));
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

    fn stmt_end(&mut self) {
        self.instructions.push(Instruction::StmtEnd);
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
                self.push(Value::Identifier(i.clone()));
            }
            ast::Expr::Assign(ident, expr) => {
                self.expr(&expr.value);
                self.assign(ident.value.clone());
            }
            ast::Expr::Call(_, items) => todo!(),
            ast::Expr::If(if_expr) => todo!(),
            ast::Expr::Block(stmts) => self.block(&stmts),
        }
    }

    fn block(&mut self, stmts: &[WithSpan<ast::Stmt>]) {
        self.scope_start();
        for stmt in stmts {
            self.stmt(&stmt.value);
        }
        if stmts
            .last()
            .map(|l| matches!(&l.value, &ast::Stmt::Expr(_)))
            .unwrap_or(true)
        {
            self.push(Value::Nil);
        }
        self.scope_end();
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
            ast::Stmt::Expr(stmt_expr) => self.expr(&stmt_expr.expr),
            ast::Stmt::Item(item) => {}
            ast::Stmt::Binding(binding) => {
                for (i, ident) in binding.identifiers.iter().enumerate() {
                    if let Some(expr) = binding.values.as_ref().map(|v| &v[i]) {
                        self.expr(&expr.value);
                    } else {
                        self.push(Value::Nil);
                    }
                    self.local(ident.value.clone());
                }
            }
            ast::Stmt::Print(v) => {
                self.expr(&v.value);
                self.instructions.push(Instruction::Print);
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
