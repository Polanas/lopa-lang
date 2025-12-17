use crate::{ast, ir, position};

pub type Identifier = String;

#[derive(Clone, Debug)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Nil,
    Identifier(Identifier),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mult,
    Div,
    Modulo,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    NotEqual,
    Equal,
    And,
    Or,
}

impl BinaryOp {
    pub fn to_lua(&self) -> &str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mult => "*",
            BinaryOp::Div => "-",
            BinaryOp::Modulo => "%",
            BinaryOp::Greater => ">",
            BinaryOp::GreaterEqual => ">=",
            BinaryOp::Less => "<",
            BinaryOp::LessEqual => "<=",
            BinaryOp::NotEqual => "~=",
            BinaryOp::Equal => "==",
            BinaryOp::And => "and",
            BinaryOp::Or => "or",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum UnaryOp {
    Minus,
    Not,
}

impl UnaryOp {
    pub fn to_lua(&self) -> &str {
        match self {
            UnaryOp::Minus => "-",
            UnaryOp::Not => "not ",
        }
    }
}

#[derive(Clone, Debug)]
pub enum OpCode {
    Push(Value),
    Store(Identifier),
    Unary(UnaryOp),
    Binary(BinaryOp),
    Binding(Identifier),
    Print,
    StmtStart,
    StmtEnd,
    BlockStart,
    BlockEnd,
}

pub struct IRConverter {
    opcodes: Vec<OpCode>,
}

impl IRConverter {
    fn new() -> Self {
        Self { opcodes: vec![] }
    }

    fn add_opcode(&mut self, opcode: OpCode) {
        self.opcodes.push(opcode)
    }

    fn push(&mut self, value: Value) {
        self.add_opcode(OpCode::Push(value));
    }

    fn store(&mut self, identifier: Identifier) {
        self.add_opcode(OpCode::Store(identifier));
    }

    fn unary(&mut self, op: UnaryOp) {
        self.add_opcode(OpCode::Unary(op));
    }

    fn binary(&mut self, op: BinaryOp) {
        self.add_opcode(OpCode::Binary(op));
    }

    fn binding(&mut self, identifier: Identifier) {
        self.add_opcode(OpCode::Binding(identifier));
    }

    fn print(&mut self) {
        self.add_opcode(OpCode::Print);
    }

    fn stmt_end(&mut self) {
        self.add_opcode(OpCode::StmtEnd);
    }

    fn stmt_start(&mut self) {
        self.add_opcode(OpCode::StmtStart);
    }

    fn block_start(&mut self) {
        self.add_opcode(OpCode::BlockStart);
    }
    fn block_end(&mut self) {
        self.add_opcode(OpCode::BlockEnd);
    }

    fn program(&mut self, program: &[position::WithSpan<ast::Stmt>]) {
        for stmt in program {
            self.stmt_start();
            match &stmt.value {
                ast::Stmt::Expr(expr) => self.expr(expr),
                ast::Stmt::Binding(binding) => {
                    for (i, ident) in binding.identifiers.iter().enumerate() {
                        self.binding(ident.value.clone());
                        if let Some(expr) = binding.values.as_ref().map(|values| &values[i]) {
                            self.expr(&expr.value);
                            self.store(ident.value.clone());
                        }
                    }
                }
                ast::Stmt::Print(expr) => {
                    self.expr(&expr.value);
                    self.print();
                }
                ast::Stmt::Item(item) => {}
            }
            self.stmt_end();
        }
    }

    fn expr(&mut self, expr: &ast::Expr) {
        match expr {
            ast::Expr::Nil => self.push(Value::Nil),
            ast::Expr::Number(number) => self.push(match number {
                ast::Number::Float(f) => Value::Float(*f),
                ast::Number::Int(i) => Value::Int(*i),
            }),
            ast::Expr::Bool(b) => self.push(Value::Bool(*b)),
            ast::Expr::String(s) => self.push(Value::String(s.clone())),
            ast::Expr::Grouping(e) => self.expr(&e.value),
            ast::Expr::Unary(op, expr) => {
                self.expr(&expr.value);
                self.unary(match &op.value {
                    ast::UnaryOp::Minus => UnaryOp::Minus,
                    ast::UnaryOp::Not => UnaryOp::Not,
                });
            }
            ast::Expr::Binary(binary_expr) => {
                self.expr(&binary_expr.left.value);
                self.expr(&binary_expr.right.value);
                self.binary(match &binary_expr.op.value {
                    ast::BinaryOperator::Div => BinaryOp::Div,
                    ast::BinaryOperator::Mult => BinaryOp::Mult,
                    ast::BinaryOperator::Add => BinaryOp::Add,
                    ast::BinaryOperator::Sub => BinaryOp::Sub,
                    ast::BinaryOperator::Greater => BinaryOp::Greater,
                    ast::BinaryOperator::GreaterEqual => BinaryOp::GreaterEqual,
                    ast::BinaryOperator::Less => BinaryOp::Less,
                    ast::BinaryOperator::LessEqual => BinaryOp::LessEqual,
                    ast::BinaryOperator::NotEqual => BinaryOp::NotEqual,
                    ast::BinaryOperator::Equal => BinaryOp::Equal,
                    ast::BinaryOperator::Modulo => BinaryOp::Modulo,
                    ast::BinaryOperator::And => BinaryOp::And,
                    ast::BinaryOperator::Or => BinaryOp::Or,
                });
            }
            ast::Expr::Identifier(i) => self.push(Value::Identifier(i.clone())),
            ast::Expr::Block(stmts) => {
                self.block_start();
                self.program(stmts);
                self.block_end();
            }
            ast::Expr::Assign(idents, values) => {
                for (ident, expr) in idents.iter().zip(values) {
                    self.expr(&expr.value);
                    self.store(ident.value.clone());
                }
            }
            ast::Expr::Call(_, items) => todo!(),
            ast::Expr::If(if_expr) => todo!(),
            ast::Expr::List(items) => todo!(),
        }
    }
}

pub fn convert(program: &[position::WithSpan<ast::Stmt>]) -> Vec<OpCode> {
    let mut ir_converter = IRConverter::new();
    ir_converter.program(program);
    ir_converter.opcodes
}
