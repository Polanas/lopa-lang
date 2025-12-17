use crate::{
    ast,
    ir::{self, OpCode::StmtEnd},
    position,
};

pub struct CodeGenerator {
    stacks: Vec<usize>,
    output: String,
}

impl CodeGenerator {
    fn new() -> Self {
        Self {
            stacks: vec![0],
            output: String::new(),
        }
    }

    fn head_mut(&mut self) -> &mut usize {
        let len = self.stacks.len();
        &mut self.stacks[len - 1]
    }

    fn head(&self) -> usize {
        let len = self.stacks.len();
        self.stacks[len - 1]
    }

    fn push(&mut self) {
        *self.head_mut() += 1
    }

    fn pop(&mut self) {
        *self.head_mut() -= 1
    }

    fn push_stack(&mut self) {
        self.stacks.push(self.head())
    }

    fn pop_stack(&mut self) {
        self.stacks.pop();
    }

    fn ident(&self, num: usize) -> String {
        format!("__stack_ident__{num}")
    }

    fn head_ident(&self) -> String {
        self.ident(self.head())
    }

    fn push_ident(&mut self) -> String {
        let ident = self.head_ident();
        self.push();
        ident
    }

    fn pop_ident(&mut self) -> String {
        self.pop();
        self.head_ident()
    }

    fn line(&mut self, text: &str) {
        self.output.push_str(text);
        self.output.push('\n');
    }

    fn program(&mut self, program: &[position::WithSpan<ast::Stmt>]) {
        let ir_opcodes = ir::convert(program);
        // dbg!(&ir_opcodes);

        for (_i, opcode) in ir_opcodes.iter().enumerate() {
            // dbg!(i, opcode);
            match opcode {
                ir::OpCode::Push(value) => match value {
                    ir::Value::Int(i) => {
                        let ident = self.push_ident();
                        self.line(&format!("local {ident} = {i}"));
                    }
                    ir::Value::Float(f) => {
                        let ident = self.push_ident();
                        self.line(&format!("local {ident} = {f}"));
                    }
                    ir::Value::String(s) => {
                        let ident = self.push_ident();
                        self.line(&format!("local {ident} = \"{s}\""));
                    }
                    ir::Value::Bool(b) => {
                        let ident = self.push_ident();
                        self.line(&format!("local {ident} = {b}"));
                    }
                    ir::Value::Nil => {
                        let ident = self.push_ident();
                        self.line(&format!("local {ident} = nil"));
                    }
                    ir::Value::Identifier(i) => {
                        let ident = self.push_ident();
                        self.line(&format!("local {ident} = {i}"));
                    }
                },
                ir::OpCode::Store(i) => {
                    let ident = self.pop_ident();
                    self.line(&format!("{i} = {ident}"));
                }
                ir::OpCode::Unary(unary_op) => {
                    let head = self.head_ident();
                    self.line(&format!("{head} = {}{head}", unary_op.to_lua()));
                }
                ir::OpCode::Binary(binary_op) => {
                    let right = self.pop_ident();
                    let ident = self.ident(self.head() - 1);
                    self.line(&format!("{ident} = {ident} {} {right}", binary_op.to_lua()));
                }
                ir::OpCode::Binding(binding) => {
                    self.line(&format!("local {binding}"));
                }
                ir::OpCode::Print => {
                    let ident = self.pop_ident();
                    self.line(&format!("print({ident})"));
                }
                ir::OpCode::StmtStart => self.push_stack(),
                ir::OpCode::StmtEnd => self.pop_stack(),
                ir::OpCode::BlockStart => self.push_stack(),
                ir::OpCode::BlockEnd => {
                    self.pop_stack();
                    self.push();
                }
            }
        }
    }
}

pub fn generate(program: &[position::WithSpan<ast::Stmt>]) -> String {
    let mut generator = CodeGenerator::new();
    generator.program(program);
    generator.output
}
//
// struct State<'a> {
//     output: &'a mut String,
//     block_offset: usize,
//     block_result_local: Option<String>,
// }
//
// fn generate_statement(stmt: &position::WithSpan<ast::Stmt>, state: &mut State) {
//     match &stmt.value {
//         ast::Stmt::Expr(expr) => {
//             if let Some(local) = &state.block_result_local {
//                 state.output.push_str(local);
//                 state.output.push_str("= ");
//             } else {
//                 state.output.push_str("return ");
//             }
//             generate_expr(&position::WithSpan::new(*expr.clone(), stmt.span), state);
//         }
//         ast::Stmt::Item(item) => {}
//         ast::Stmt::Binding(binding) => {
//             state.output.push_str(match binding.binding_type {
//                 ast::BindingType::Let => "local ",
//                 ast::BindingType::Global => "",
//             });
//             state.output.push_str(&binding.identifiers[0].value);
//             state.output.push('\n');
//             if let Some(values) = &binding.values {
//                 state.output.push_str(&binding.identifiers[0].value);
//                 state.output.push_str(" = ");
//
//                 generate_expr(&values[0], state);
//             }
//         }
//         ast::Stmt::Print(expr) => {
//             state.output.push_str("print(");
//             generate_expr(expr, state);
//             state.output.push(')');
//         }
//     }
// }
//
// fn generate_expr(expr: &position::WithSpan<ast::Expr>, state: &mut State) {
//     match &expr.value {
//         ast::Expr::Nil => state.output.push_str("nil"),
//         ast::Expr::Number(number) => match number {
//             ast::Number::Float(f) => state.output.push_str(&f.to_string()),
//             ast::Number::Int(i) => state.output.push_str(&i.to_string()),
//         },
//         ast::Expr::Bool(b) => state.output.push_str(&b.to_string()),
//         ast::Expr::String(str) => {
//             state.output.push('"');
//             state.output.push_str(str);
//             state.output.push('"');
//         }
//         ast::Expr::Grouping(e) => {
//             state.output.push('(');
//             generate_expr(e, state);
//             state.output.push(')');
//         }
//         ast::Expr::Unary(op, expr) => {
//             match op.value {
//                 ast::UnaryOp::Minus => state.output.push('-'),
//                 ast::UnaryOp::Not => state.output.push_str("not "),
//             };
//             generate_expr(expr, state);
//         }
//         ast::Expr::Binary(binary_expr) => {
//             generate_expr(&binary_expr.left, state);
//             state.output.push_str(binary_expr.op.value.to_lua());
//             generate_expr(&binary_expr.right, state);
//         }
//         ast::Expr::Identifier(identifier) => state.output.push_str(identifier),
//         ast::Expr::Block(items) => {
//             let block_local_name = format!("__local__{}", state.output.len());
//             let offset = state.block_offset;
//             let declaration_string = format!(" local {block_local_name} do");
//             state.block_offset += declaration_string.len();
//             state.output.insert_str(offset, &declaration_string);
//             state.block_result_local = Some(declaration_string);
//
//             let block_code = generate_program(items);
//             // for stmt in items {
//             //     let mut output = String::new();
//             //     let mut state = State {
//             //         output: &mut output,
//             //         block_offset: 0,
//             //     };
//             //     generate_statement();
//             // }
//
//             // state.output.push_str(" end");
//             // generate_expr();
//             state.block_result_local = None;
//         }
//         ast::Expr::Assign(items, items1) => todo!(),
//         ast::Expr::Call(_, items) => todo!(),
//         ast::Expr::If(if_expr) => todo!(),
//         ast::Expr::List(items) => todo!(),
//     }
// }
