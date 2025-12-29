use crate::common::*;
use crate::instruction as I;
use crate::{ir, luajit};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
struct SlotId(pub usize);

type Identifier = String;

#[derive(Debug, Clone)]
struct Scope {
    locals: Vec<Identifier>,
    stack: Vec<Option<Value>>,
}

impl Scope {
    fn new() -> Self {
        Self {
            locals: Default::default(),
            stack: Default::default(),
        }
    }

    fn local(&mut self, ident: &str) -> usize {
        if let Some(id) = self.locals.iter().position(|v| v == ident) {
            id
        } else {
            let id = self.locals.len();
            self.locals.push(ident.to_owned());
            self.stack.insert(id, Some(Value::Local));
            id
        }
    }

    fn push(&mut self, value: Option<Value>) -> usize {
        let id = self.stack.len();
        self.stack.push(value);
        id
    }

    fn pop(&mut self) -> (usize, Option<Value>) {
        let value = self.stack.pop().unwrap();
        (self.stack.len(), value)
    }

    fn clear_temp(&mut self) {
        self.stack.drain(self.locals.len()..self.stack.len());
    }

    fn top(&self) -> usize {
        self.stack.len() - 1
    }
}
#[derive(Clone, Debug, Copy)]
pub enum Value {
    Int16(i16),
    Number(ir::ConstantId),
    String(ir::ConstantId),
    Global(ir::ConstantId),
    Bool(bool),
    Nil,
    Local,
}

pub struct Context {
    luajit_context: luajit::Context,
    scopes: Vec<Scope>,
    persistent_stack: usize,
}

impl Context {
    fn new() -> Self {
        Self {
            luajit_context: luajit::Context::new(),
            scopes: Default::default(),
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

    fn try_move(dst: u8, src: u16, proto: &mut luajit::Proto) {
        if dst != (src as u8) {
            proto.instructions.push(I!(MOV, dst, src))
        }
    }

    fn generate(&mut self, mut ir_context: ir::FunctionContext) {
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
            let scope = self.current_scope_mut();
            match instruction {
                ir::Instruction::Push(value) => match value {
                    &ir::Value::Int16(id) => proto.instructions.push(I!(
                        KSHORT,
                        scope.push(Some(Value::Int16(id))) as _,
                        id.cast_unsigned()
                    )),
                    &ir::Value::Number(id) => {
                        proto.instructions.push(I!(
                            KNUM,
                            scope.push(Some(Value::Number(id))) as _,
                            id.0
                        ));
                    }
                    &ir::Value::String(id) => {
                        proto.instructions.push(I!(
                            KSTR,
                            scope.push(Some(Value::String(id))) as _,
                            id.0
                        ));
                    }
                    &ir::Value::Bool(b) => proto.instructions.push(I!(
                        KPRI,
                        scope.push(Some(Value::Bool(b))) as _,
                        if b { 2 } else { 1 }
                    )),
                    ir::Value::Nil => {
                        proto
                            .instructions
                            .push(I!(KPRI, scope.push(Some(Value::Nil)) as _, 0))
                    }
                    ir::Value::Table(table) => todo!(),
                    ir::Value::Local(i) => {
                        let id = scope.locals.iter().position(|l| l == i).unwrap();
                        proto.instructions.push(I!(
                            MOV,
                            scope.push(Some(Value::Local)) as _,
                            id as _
                        ));
                    }
                    &ir::Value::Global(id) => {
                        let id = if let luajit::GCConstant::Str(s) =
                            &proto.gc_constants[id.0 as usize]
                        {
                            ir_context.string_constants.get(s).unwrap()
                        } else {
                            panic!("expected string");
                        };
                        proto.instructions.push(I!(
                            GGET,
                            scope.push(Some(Value::Global(*id))) as _,
                            id.0 as _
                        ))
                    }
                },
                ir::Instruction::Pop => {
                    self.current_scope_mut().pop();
                }
                ir::Instruction::Locals(idents) => {
                    let locals_amount = scope.locals.len();
                    let scope = self.current_scope_mut();
                    for (i, ident) in idents.iter().enumerate() {
                        let local_id = scope.local(ident);
                        let id = i + locals_amount;
                        Self::try_move(local_id as _, id as _, &mut proto);
                    }
                    scope.clear_temp();
                }
                ir::Instruction::Assign(idents) => {
                    let scope = self.current_scope_mut();
                    for (i, ident) in idents.iter().enumerate() {
                        let id = i + scope.locals.len();

                        if scope.locals.contains(ident) {
                            let local_id = scope.local(ident);
                            Self::try_move(local_id as _, id as _, &mut proto);
                        } else {
                            let const_id = ir_context.string_constants.get(ident).unwrap().0;
                            proto.instructions.push(I!(GSET, id as _, const_id))
                        }
                    }
                    scope.clear_temp();
                }
                ir::Instruction::Unary(unary_op) => {
                    let id = scope.pop().0;
                    match unary_op {
                        UnaryOp::Not => {
                            proto
                                .instructions
                                .push(I!(NOT, scope.push(None) as _, id as _));
                        }
                        UnaryOp::Negate => {
                            proto
                                .instructions
                                .push(I!(UNM, scope.push(None) as _, id as _));
                        }
                    }
                }
                ir::Instruction::Binary(binary_op) => {
                    let scope = self.current_scope_mut();
                    let right_id = scope.pop().0;
                    let left_id = scope.pop().0;

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

                    proto.instructions.push(luajit::Instruction::ABC(
                        opcode,
                        luajit::ABC::new(scope.push(None) as _, left_id as _, right_id as _),
                    ));
                }
                ir::Instruction::Jump(conditional_jump, _) => todo!(),
                ir::Instruction::StmtEnd => {
                    // self.clear_temp();
                }
                ir::Instruction::ScopeStart => {
                    self.push_scope();
                }
                ir::Instruction::ScopeEnd => {
                    self.pop_scope();
                }
                ir::Instruction::Print => {
                    let scope = self.current_scope_mut();
                    let id = scope.top();

                    proto
                        .instructions
                        .push(I!(GGET, scope.push(None) as _, print_const));
                    let print = scope.top();

                    scope.push(None);
                    Self::try_move(scope.push(None) as _, id as _, &mut proto);
                    proto.instructions.push(I!(CALL, print as _, 1, 2));

                    scope.pop();
                    scope.pop();
                    scope.pop();
                    scope.pop();
                }
                ir::Instruction::Global(global) => todo!(),
            }
        }
        self.pop_scope();
        proto.instructions.push(I!(RET0, 0, 1));

        proto.gc_constants.reverse();
        proto.number_constants.reverse();

        self.luajit_context.write_proto(proto);
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

pub fn generate(ir: ir::FunctionContext) -> Vec<u8> {
    let mut context = Context::new();
    context.generate(ir);
    context.luajit_context.finish()
}
