use crate::common::*;
use crate::instruction as I;
use crate::{ir_new, luajit};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
struct SlotId(pub usize);

type Identifier = String;

#[derive(Debug, Clone)]
struct Scope {
    locals: HashMap<Identifier, SlotId>,
    stack: usize,
}

impl Scope {
    fn new() -> Self {
        Self {
            locals: Default::default(),
            stack: 0,
        }
    }

    fn local(&mut self, ident: &str) -> usize {
        if let Some(slot) = self.locals.get(ident) {
            slot.0
        } else {
            let stack = self.stack;
            self.locals.insert(ident.to_owned(), SlotId(stack));
            self.stack += 1;
            stack
        }
    }
}

pub struct Context {
    luajit_context: luajit::Context,
    scopes: Vec<Scope>,
    temp_stack: usize,
    persistent_stack: usize,
}

impl Context {
    fn new() -> Self {
        Self {
            luajit_context: luajit::Context::new(),
            scopes: Default::default(),
            temp_stack: 0,
            persistent_stack: 0,
        }
    }

    fn push_persistent(&mut self) -> usize {
        let stack = self.persistent_stack;
        self.persistent_stack += 1;
        stack
    }

    fn pop_persistent(&mut self) -> usize {
        let stack = self.persistent_stack;
        self.persistent_stack -= 1;
        stack
    }

    fn push_temp(&mut self) -> usize {
        let stack = self.temp_stack;
        self.temp_stack += 1;
        stack + self.persistent_stack + self.current_scope_mut().stack
    }

    fn pop_temp(&mut self) -> usize {
        self.temp_stack -= 1;
        self.temp_stack + self.persistent_stack + self.current_scope_mut().stack
    }

    fn clear_temp(&mut self) {
        self.temp_stack = 0;
    }

    fn top_temp(&self) -> usize {
        self.temp_stack + self.persistent_stack - 1 + self.current_scope().stack
    }

    fn top_temp_optional(&self) -> Option<usize> {
        if self.temp_stack == 0 {
            None
        } else {
            Some(self.temp_stack + self.persistent_stack - 1 + self.current_scope().stack)
        }
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        let len = self.scopes.len() - 1;
        &mut self.scopes[len]
    }

    fn current_scope(&self) -> &Scope {
        let len = self.scopes.len() - 1;
        &self.scopes[len]
    }

    fn push_scope(&mut self) -> &mut Scope {
        if self.scopes.is_empty() {
            self.scopes.push(Scope::new());
        }
        let scope = self.current_scope_mut().clone();
        self.scopes.push(scope);
        self.current_scope_mut()
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn move_if_different(a: u8, d: u16, proto: &mut luajit::Proto) {
        if a != (d as u8) {
            proto.instructions.push(I!(MOV, a, d))
        }
    }

    fn generate(&mut self, mut ir_context: ir_new::FunctionContext) {
        self.push_scope();
        ir_context
            .gc_constants
            .push(luajit::GCConstant::Str(String::from("print")));
        let print_const = (ir_context.gc_constants.len() - 1) as u16;
        let mut proto = luajit::Proto::default();

        for gc_const in ir_context.gc_constants {
            proto.gc_constants.push(gc_const);
        }
        for num_const in ir_context.num_constants {
            proto.number_constants.push(num_const);
        }

        for (id, instruction) in ir_context.instructions.iter().enumerate() {
            match instruction {
                ir_new::Instruction::Push(value) => match value {
                    ir_new::Value::Int16(id) => proto.instructions.push(I!(
                        KSHORT,
                        self.push_temp() as _,
                        id.cast_unsigned()
                    )),
                    ir_new::Value::Number(id) => {
                        proto
                            .instructions
                            .push(I!(KNUM, self.push_temp() as _, id.0));
                    }
                    ir_new::Value::String(id) => {
                        proto
                            .instructions
                            .push(I!(KSTR, self.push_temp() as _, id.0));
                    }
                    ir_new::Value::Bool(b) => proto.instructions.push(I!(
                        KPRI,
                        self.push_temp() as _,
                        if *b { 2 } else { 1 }
                    )),
                    ir_new::Value::Nil => {
                        proto.instructions.push(I!(KPRI, self.push_temp() as _, 0))
                    }
                    ir_new::Value::Table(table) => todo!(),
                    ir_new::Value::Identifier(i) => (),
                },
                ir_new::Instruction::Pop => {
                    self.pop_temp();
                }
                ir_new::Instruction::Local(ident) => {
                    let id = self.pop_temp();
                    let local = self.current_scope_mut().local(ident);
                    Self::move_if_different(local as _, id as _, &mut proto);
                }
                ir_new::Instruction::Assign(ident) => {
                    let id = self.pop_temp();
                    let local = self.current_scope_mut().local(ident);
                    Self::move_if_different(local as _, id as _, &mut proto);
                }
                ir_new::Instruction::Unary(unary_op) => {
                    let prev = &ir_context.instructions[id - 1];
                    if let ir_new::Instruction::Push(ir_new::Value::Identifier(i)) = prev {
                        let local = self.current_scope_mut().local(i);
                        match unary_op {
                            UnaryOp::Not => {
                                proto
                                    .instructions
                                    .push(I!(NOT, self.push_temp() as _, local as _));
                            }
                            UnaryOp::Negate => {
                                proto
                                    .instructions
                                    .push(I!(UNM, self.push_temp() as _, local as _));
                            }
                        }
                    } else {
                        match unary_op {
                            UnaryOp::Not => {
                                proto.instructions.push(I!(
                                    NOT,
                                    self.top_temp() as _,
                                    self.top_temp() as _,
                                ));
                            }
                            UnaryOp::Negate => {
                                proto.instructions.push(I!(
                                    UNM,
                                    self.top_temp() as _,
                                    self.top_temp() as _,
                                ));
                            }
                        }
                    }
                }
                ir_new::Instruction::Binary(binary_op) => {
                    let right = if let ir_new::Instruction::Push(ir_new::Value::Identifier(i)) =
                        &ir_context.instructions[id - 1]
                    {
                        self.current_scope_mut().local(i)
                    } else {
                        self.pop_temp()
                    };
                    let left = if let ir_new::Instruction::Push(ir_new::Value::Identifier(i)) =
                        &ir_context.instructions[id - 2]
                    {
                        self.current_scope_mut().local(i)
                    } else {
                        self.pop_temp()
                    };

                    let opcode = match binary_op {
                        BinaryOp::Div => luajit::OpCode::DIVVV,
                        BinaryOp::Mult => luajit::OpCode::MULVV,
                        BinaryOp::Add => luajit::OpCode::ADDVV,
                        BinaryOp::Sub => luajit::OpCode::SUBVV,
                        BinaryOp::Greater => luajit::OpCode::KSHORT,
                        BinaryOp::GreaterEqual => todo!(),
                        BinaryOp::Less => todo!(),
                        BinaryOp::LessEqual => todo!(),
                        BinaryOp::NotEqual => todo!(),
                        BinaryOp::Equal => todo!(),
                        BinaryOp::Modulo => luajit::OpCode::MODVV,
                        BinaryOp::And => todo!(),
                        BinaryOp::Or => todo!(),
                    };

                    //TODO: implement assign directly to locals
                    proto.instructions.push(luajit::Instruction::ABC(
                        opcode,
                        luajit::ABC::new(self.push_temp() as _, left as _, right as _),
                    ));
                }
                ir_new::Instruction::Jump(conditional_jump, _) => todo!(),
                ir_new::Instruction::StmtEnd => self.clear_temp(),
                ir_new::Instruction::ScopeStart => {}
                ir_new::Instruction::ScopeEnd => {}
                ir_new::Instruction::Print => {
                    let top = self.top_temp_optional();

                    proto
                        .instructions
                        .push(I!(GGET, self.push_temp() as _, print_const));
                    let print = self.top_temp();

                    self.push_temp();
                    let prev = &ir_context.instructions[id - 1];
                    if let ir_new::Instruction::Push(ir_new::Value::Identifier(i)) = prev {
                        let local = self.current_scope_mut().local(i);
                        Self::move_if_different(self.push_temp() as _, local as _, &mut proto);
                        proto.instructions.push(I!(CALL, print as _, 1, 2));
                    } else {
                        Self::move_if_different(
                            self.push_temp() as _,
                            top.unwrap() as _,
                            &mut proto,
                        );
                        proto.instructions.push(I!(CALL, print as _, 1, 2));
                    }

                    self.pop_temp();
                    self.pop_temp();
                    self.pop_temp();
                }
                ir_new::Instruction::Global(global) => todo!(),
            }
        }
        self.pop_scope();
        proto.instructions.push(I!(RET0, 0, 1));

        proto.gc_constants.reverse();
        proto.number_constants.reverse();

        self.luajit_context.write_proto(proto);
    }
}

pub fn generate(ir: ir_new::FunctionContext) -> Vec<u8> {
    let mut context = Context::new();
    context.generate(ir);
    context.luajit_context.finish()
}
