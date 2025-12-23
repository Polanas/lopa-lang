// use crate::{ast, ir, position};
//
// type Identifier = String;
// type Label = String;
//
// #[derive(Clone, Debug)]
// pub enum FnArg {
//     Identifier(Identifier),
//     VarArgs,
// }
//
// impl std::fmt::Display for FnArg {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             FnArg::Identifier(ident) => write!(f, "{ident}"),
//             FnArg::VarArgs => write!(f, "..."),
//         }
//     }
// }
//
// #[derive(Clone, Debug)]
// pub enum Value {
//     Int(i64),
//     Float(f64),
//     String(String),
//     Bool(bool),
//     Nil,
//     Identifier(Identifier),
//     Table,
//     Fn(Vec<FnArg>, Vec<Instruction>),
//     Call(usize),
//     Get,
// }
//
// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
// pub enum BinaryOp {
//     Add,
//     Sub,
//     Mult,
//     Div,
//     Modulo,
//     Greater,
//     GreaterEqual,
//     Less,
//     LessEqual,
//     NotEqual,
//     Equal,
//     And,
//     Or,
// }
//
// impl std::fmt::Display for BinaryOp {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             BinaryOp::Add => write!(f, "+"),
//             BinaryOp::Sub => write!(f, "-"),
//             BinaryOp::Mult => write!(f, "*"),
//             BinaryOp::Div => write!(f, "/"),
//             BinaryOp::Modulo => write!(f, "%"),
//             BinaryOp::Greater => write!(f, ">"),
//             BinaryOp::GreaterEqual => write!(f, ">="),
//             BinaryOp::Less => write!(f, "<"),
//             BinaryOp::LessEqual => write!(f, "<="),
//             BinaryOp::NotEqual => write!(f, "~="),
//             BinaryOp::Equal => write!(f, "=="),
//             BinaryOp::And => write!(f, "and"),
//             BinaryOp::Or => write!(f, "or"),
//         }
//     }
// }
//
// #[derive(Clone, Copy, Debug)]
// pub enum UnaryOp {
//     Minus,
//     Not,
// }
//
// impl std::fmt::Display for UnaryOp {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             UnaryOp::Minus => write!(f, "-"),
//             UnaryOp::Not => write!(f, "not "),
//         }
//     }
// }
//
// #[derive(Clone, Copy, Debug)]
// pub enum BindingType {
//     Local,
//     Global,
// }
//
// impl std::fmt::Display for BindingType {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             BindingType::Local => write!(f, "local"),
//             BindingType::Global => write!(f, ""),
//         }
//     }
// }
//
// #[derive(Clone, Debug)]
// pub enum Instruction {
//     Push(Value),
//     Pop,
//     Store(Identifier),
//     Unary(UnaryOp),
//     Binary(BinaryOp),
//     Binding(BindingType, Identifier),
//     Assign(Identifier),
//     Label(Label),
//     Jump(Label),
//     JumpIfEqual(Label),
//     JumpIfNotEqual(Label),
//     Return,
//     Set,
//     Print,
//     StmtStart,
//     StmtEnd,
//     BlockStart,
//     BlockEnd,
// }
//
// const LABEL_IDENT: &str = "__label__ident__";
//
// pub struct IRConverter {
//     instructions: Vec<Instruction>,
//     labels_amount: usize,
// }
//
// impl IRConverter {
//     fn new() -> Self {
//         Self {
//             instructions: vec![],
//             labels_amount: 0,
//         }
//     }
//
//     fn add_opcode(&mut self, opcode: Instruction) {
//         self.instructions.push(opcode)
//     }
//
//     fn pop(&mut self) {
//         self.add_opcode(Instruction::Pop);
//     }
//
//     fn push(&mut self, value: Value) {
//         self.add_opcode(Instruction::Push(value));
//     }
//
//     fn store(&mut self, identifier: Identifier) {
//         self.add_opcode(Instruction::Store(identifier));
//     }
//
//     fn unary(&mut self, op: UnaryOp) {
//         self.add_opcode(Instruction::Unary(op));
//     }
//
//     fn binary(&mut self, op: BinaryOp) {
//         self.add_opcode(Instruction::Binary(op));
//     }
//
//     fn binding(&mut self, binding_type: BindingType, identifier: Identifier) {
//         self.add_opcode(Instruction::Binding(binding_type, identifier));
//     }
//
//     fn print(&mut self) {
//         self.add_opcode(Instruction::Print);
//     }
//
//     fn stmt_end(&mut self) {
//         self.add_opcode(Instruction::StmtEnd);
//     }
//
//     fn stmt_start(&mut self) {
//         self.add_opcode(Instruction::StmtStart);
//     }
//
//     fn block_start(&mut self) {
//         self.add_opcode(Instruction::BlockStart);
//     }
//
//     fn block_end(&mut self) {
//         self.add_opcode(Instruction::BlockEnd);
//     }
//
//     fn label(&mut self, label: Label) {
//         self.add_opcode(Instruction::Label(label));
//     }
//
//     fn jump(&mut self, label: Label) {
//         self.add_opcode(Instruction::Jump(label));
//     }
//
//     fn jump_ine(&mut self, label: Label) {
//         self.add_opcode(Instruction::JumpIfNotEqual(label));ir
//     }
//
//     fn jump_ie(&mut self, label: Label) {
//         self.add_opcode(Instruction::JumpIfEqual(label));
//     }
//
//     fn label_ident(&mut self) -> Label {
//         let ident = format!("{LABEL_IDENT}{}", self.labels_amount);
//         self.labels_amount += 1;
//         ident
//     }
//
//     fn program(&mut self, program: &[position::WithSpan<ast::Stmt>]) {
//         for stmt in program {
//             self.stmt_start();
//             self.stmt(stmt);
//             self.stmt_end();
//         }
//     }
//
//     fn block(&mut self, program: &[position::WithSpan<ast::Stmt>]) {
//         self.block_start();
//         if program.is_empty() {
//             self.push(Value::Nil);
//             self.block_end();
//             return;
//         }
//         for stmt in program {
//             self.stmt_start();
//             self.stmt(stmt);
//             self.stmt_end();
//         }
//         self.block_end();
//     }
//
//     fn stmt(&mut self, stmt: &position::WithSpan<ast::Stmt>) {
//         match &stmt.value {
//             ast::Stmt::Expr(ast::StmtExpr { expr, semi }) => {
//                 self.expr(expr);
//                 if semi.is_some() {
//                     self.pop();
//                 }
//             }
//             ast::Stmt::Binding(binding) => {
//                 for (i, ident) in binding.identifiers.iter().enumerate() {
//                     self.binding(
//                         match binding.binding_type {
//                             ast::BindingType::Local => BindingType::Local,
//                             ast::BindingType::Global => BindingType::Global,
//                         },
//                         ident.value.clone(),
//                     );
//                     if let Some(expr) = binding.values.as_ref().map(|values| &values[i]) {
//                         self.expr(&expr.value);
//                         self.store(ident.value.clone());
//                     }
//                 }
//             }
//             ast::Stmt::Print(expr) => {
//                 self.expr(&expr.value);
//                 self.print();
//             }
//             ast::Stmt::Item(item) => {}
//         }
//     }
//
//     fn expr(&mut self, expr: &ast::Expr) {
//         match expr {
//             ast::Expr::Nil => self.push(Value::Nil),
//             ast::Expr::Number(number) => self.push(match number {
//                 ast::Number::Float(f) => Value::Float(*f),
//                 ast::Number::Int(i) => Value::Int(*i),
//             }),
//             ast::Expr::Bool(b) => self.push(Value::Bool(*b)),
//             ast::Expr::String(s) => self.push(Value::String(s.clone())),
//             ast::Expr::Grouping(e) => self.expr(&e.value),
//             ast::Expr::Unary(op, expr) => {
//                 self.expr(&expr.value);
//                 self.unary(match &op.value {
//                     ast::UnaryOp::Negate => UnaryOp::Minus,
//                     ast::UnaryOp::Not => UnaryOp::Not,
//                 });
//             }
//             ast::Expr::Binary(binary_expr) => {
//                 self.expr(&binary_expr.left.value);
//                 self.expr(&binary_expr.right.value);
//                 self.binary(match &binary_expr.op.value {
//                     ast::BinaryOp::Div => BinaryOp::Div,
//                     ast::BinaryOp::Mult => BinaryOp::Mult,
//                     ast::BinaryOp::Add => BinaryOp::Add,
//                     ast::BinaryOp::Sub => BinaryOp::Sub,
//                     ast::BinaryOp::Greater => BinaryOp::Greater,
//                     ast::BinaryOp::GreaterEqual => BinaryOp::GreaterEqual,
//                     ast::BinaryOp::Less => BinaryOp::Less,
//                     ast::BinaryOp::LessEqual => BinaryOp::LessEqual,
//                     ast::BinaryOp::NotEqual => BinaryOp::NotEqual,
//                     ast::BinaryOp::Equal => BinaryOp::Equal,
//                     ast::BinaryOp::Modulo => BinaryOp::Modulo,
//                     ast::BinaryOp::And => BinaryOp::And,
//                     ast::BinaryOp::Or => BinaryOp::Or,
//                 });
//             }
//             ast::Expr::Identifier(i) => self.push(Value::Identifier(i.clone())),
//             ast::Expr::Block(stmts) => {
//                 self.block(stmts);
//             }
//             ast::Expr::Call(_, items) => todo!(),
//             ast::Expr::If(if_expr) => {
//                 self.block_start();
//                 let else_label = self.label_ident();
//                 let end_label = self.label_ident();
//                 self.expr(&if_expr.condition.value);
//                 self.jump_ine(else_label.clone());
//                 self.block(&if_expr.then_branch);
//                 self.jump(end_label.clone());
//
//                 self.label(else_label);
//
//                 if let Some(else_branch) = if_expr.else_branch.as_deref() {
//                     self.pop();
//                     self.expr(&else_branch.value);
//                 }
//
//                 self.label(end_label);
//                 self.block_end();
//             }
//             ast::Expr::Assign(_, _) => todo!(),
//         }
//     }
// }
//
// pub fn convert(program: &[position::WithSpan<ast::Stmt>]) -> Vec<Instruction> {
//     let mut ir_converter = IRConverter::new();
//     ir_converter.program(program);
//     ir_converter.instructions
// }
