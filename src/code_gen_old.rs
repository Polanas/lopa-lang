use crate::{
    ast,
    ir::{self},
    position,
};

// const STACK_IDENT: &str = "__stack__ident__";
// const STACK_LOCALS_AMOUNT: u32 = 16;
//
// pub struct CodeGenerator {
//     stacks: Vec<usize>,
//     output: String,
// }
//
// impl CodeGenerator {
//     fn new() -> Self {
//         let mut generator = Self {
//             stacks: vec![0],
//             output: String::new(),
//         };
//         generator.bind_stack_locals(STACK_LOCALS_AMOUNT);
//         generator
//     }
//
//     fn bind_stack_locals(&mut self, amount: u32) {
//         for i in 0..amount {
//             self.line(&format!("local {STACK_IDENT}{i}"));
//         }
//     }
//
//     fn head_mut(&mut self) -> &mut usize {
//         let len = self.stacks.len();
//         &mut self.stacks[len - 1]
//     }
//
//     fn head(&self) -> usize {
//         let len = self.stacks.len();
//         self.stacks[len - 1]
//     }
//
//     fn push(&mut self) {
//         *self.head_mut() += 1
//     }
//
//     fn pop(&mut self) {
//         *self.head_mut() -= 1
//     }
//
//     fn push_stack(&mut self) {
//         self.stacks.push(self.head())
//     }
//
//     fn pop_stack(&mut self) {
//         self.stacks.pop();
//     }
//
//     fn ident(&self, num: usize) -> String {
//         format!("{STACK_IDENT}{num}")
//     }
//
//     fn head_ident(&self) -> String {
//         self.ident(self.head())
//     }
//
//     fn push_ident(&mut self) -> String {
//         let ident = self.head_ident();
//         self.push();
//         ident
//     }
//
//     fn pop_ident(&mut self) -> String {
//         self.pop();
//         self.head_ident()
//     }
//
//     fn line(&mut self, text: &str) {
//         self.output.push_str(text);
//         self.output.push('\n');
//     }
//
//     fn opcode(&mut self, opcode: &ir::Instruction) {
//         match opcode {
//             ir::Instruction::Push(value) => match value {
//                 ir::Value::Int(i) => {
//                     let ident = self.push_ident();
//                     self.line(&format!("{ident} = {i}"));
//                 }
//                 ir::Value::Float(f) => {
//                     let ident = self.push_ident();
//                     self.line(&format!("{ident} = {f}"));
//                 }
//                 ir::Value::String(s) => {
//                     let ident = self.push_ident();
//                     self.line(&format!("{ident} = \"{s}\""));
//                 }
//                 ir::Value::Bool(b) => {
//                     let ident = self.push_ident();
//                     self.line(&format!("{ident} = {b}"));
//                 }
//                 ir::Value::Nil => {
//                     let ident = self.push_ident();
//                     self.line(&format!("{ident} = nil"));
//                 }
//                 ir::Value::Identifier(i) => {
//                     let ident = self.push_ident();
//                     self.line(&format!("{ident} = {i}"));
//                 }
//                 ir::Value::Table => {
//                     let ident = self.push_ident();
//                     self.line(&format!("{ident} = {{}}"));
//                 }
//                 ir::Value::Fn(fn_args, opcodes) => {
//                     let ident = self.push_ident();
//                     let mut args_string = String::new();
//                     for arg in &fn_args[0..(fn_args.len() - 1)] {
//                         args_string.push_str(&format!("{arg},"));
//                     }
//                     if let Some(last) = fn_args.last() {
//                         args_string.push_str(&last.to_string());
//                     }
//
//                     self.line(&format!("{ident} = function({args_string})"));
//                     for opcode in opcodes {
//                         self.opcode(opcode);
//                     }
//                     self.line("end");
//                 }
//                 ir::Value::Call(args_amount) => {
//                     let func = self.pop_ident();
//                     let mut args = vec![];
//                     for _ in 0..*args_amount {
//                         args.push(self.pop_ident());
//                     }
//                     let mut args_string = String::new();
//                     for arg in &args[0..args.len() - 1] {
//                         args_string.push_str(&format!("{arg},"));
//                     }
//                     if let Some(last_arg) = args.last() {
//                         args_string.push_str(&last_arg.to_string());
//                     }
//                     let ident = self.push_ident();
//                     self.line(&format!("{ident} = {func}({args_string})"));
//                 }
//                 ir::Value::Get => {
//                     let table = self.pop_ident();
//                     let ident = self.ident(self.head() - 1);
//                     self.line(&format!("{ident} = {table}[{ident}]"));
//                 }
//             },
//             ir::Instruction::Store(i) => {
//                 let ident = self.pop_ident();
//                 self.line(&format!("{i} = {ident}"));
//             }
//             ir::Instruction::Unary(unary_op) => {
//                 let ident = self.ident(self.head() - 1);
//                 self.line(&format!("{ident} = {unary_op}{ident}"));
//             }
//             ir::Instruction::Binary(binary_op) => {
//                 let right = self.pop_ident();
//                 let ident = self.ident(self.head() - 1);
//                 self.line(&format!("{ident} = {ident} {binary_op} {right}"));
//             }
//             ir::Instruction::Binding(binding_type, binding) => {
//                 self.line(&format!("{binding_type} {binding} = 0"));
//             }
//             ir::Instruction::Print => {
//                 let ident = self.pop_ident();
//                 self.line(&format!("print({ident})"));
//             }
//             ir::Instruction::StmtStart => {
//                 self.push_stack();
//             }
//             ir::Instruction::StmtEnd => {
//                 self.pop_stack();
//             }
//             ir::Instruction::BlockStart => {
//                 self.line("do");
//                 self.push_stack();
//             }
//             ir::Instruction::BlockEnd => {
//                 self.pop_stack();
//                 self.push();
//                 self.line("end");
//             }
//             ir::Instruction::Label(label) => {
//                 self.line(&format!("::{label}::"));
//             }
//             ir::Instruction::Jump(label) => {
//                 self.line(&format!("goto {label}"));
//             }
//             ir::Instruction::JumpIfEqual(label) => {
//                 let ident = self.pop_ident();
//                 self.line(&format!("if {ident} then"));
//                 self.line(&format!("goto {label}"));
//                 self.line("end");
//             }
//             ir::Instruction::JumpIfNotEqual(label) => {
//                 let ident = self.pop_ident();
//                 self.line(&format!("if not ({ident}) then"));
//                 self.line(&format!("goto {label}"));
//                 self.line("end");
//             }
//             ir::Instruction::Set => {
//                 let table = self.pop_ident();
//                 let value = self.pop_ident();
//                 let key = self.pop_ident();
//                 self.line(&format!("{table}[{key}] = {value}"))
//             }
//             ir::Instruction::Return => {
//                 let ident = self.pop_ident();
//                 self.line(&format!("return {ident}"));
//             }
//             ir::Instruction::Assign(assign_ident) => {
//                 let ident = self.pop_ident();
//                 self.line(&format!("{assign_ident} = {ident}"))
//             }
//             ir::Instruction::Pop => self.pop(),
//         }
//     }
//
//     fn program(&mut self, program: &[position::WithSpan<ast::Stmt>]) {
//         let ir_opcodes = ir::convert(program);
//         dbg!(&ir_opcodes);
//
//         for (_i, opcode) in ir_opcodes.iter().enumerate() {
//             self.opcode(opcode);
//             // dbg!(i, opcode);
//         }
//     }
// }
//
// pub fn generate(program: &[position::WithSpan<ast::Stmt>]) -> String {
//     let mut generator = CodeGenerator::new();
//     generator.program(program);
//     generator.output
// }
