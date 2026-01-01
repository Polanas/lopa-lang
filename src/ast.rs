use std::fmt::Display;

use crate::{
    common::*,
    position::{self, WithSpan},
};

pub type Identifier = String;

#[derive(Debug, PartialEq, Clone)]
pub struct BinaryExpr {
    pub left: Box<WithSpan<Expr>>,
    pub right: Box<WithSpan<Expr>>,
    pub op: WithSpan<BinaryOp>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct IfExpr {
    pub condition: Box<WithSpan<Expr>>,
    pub then_branch: Vec<WithSpan<Stmt>>,
    pub else_branch: Option<Box<WithSpan<Expr>>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FnExpr {
    name: WithSpan<Identifier>,
    // args:
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Number {
    Float(f64),
    Int(i64),
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::Float(float) => write!(f, "{float}"),
            Number::Int(i) => write!(f, "{i}"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Nil,
    Number(Number),
    Bool(bool),
    String(String),
    Grouping(Box<WithSpan<Expr>>),
    Unary(WithSpan<UnaryOp>, Box<WithSpan<Expr>>),
    Binary(BinaryExpr),
    Identifier(Identifier),
    Call(Box<WithSpan<Expr>>, Vec<WithSpan<Expr>>),
    If(IfExpr),
    Block(Vec<WithSpan<Stmt>>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Binding {
    pub kind: BindingKind,
    pub identifiers: Vec<WithSpan<Identifier>>,
    pub values: Option<Vec<WithSpan<Expr>>>,
}

impl Binding {
    pub fn as_ref(&'_ self) -> BindingRef<'_> {
        BindingRef {
            kind: self.kind,
            identifiers: &self.identifiers,
            values: self.values.as_deref(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct BindingRef<'a> {
    pub kind: BindingKind,
    pub identifiers: &'a [WithSpan<Identifier>],
    pub values: Option<&'a [WithSpan<Expr>]>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Item {}

#[derive(Debug, Clone, PartialEq)]
pub struct StmtExpr {
    pub exprs: Vec<WithSpan<Expr>>,
    pub semi: Option<position::Span>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    Expr(StmtExpr),
    Item(Item),
    Assign(Vec<WithSpan<String>>, Vec<WithSpan<Expr>>),
    Binding(Binding),
    Print(Box<WithSpan<Expr>>),
    Empty,
}

// struct DebugPrint<'a> {
//     result: String,
//     program: &'a [WithSpan<Stmt>],
// }
//
// impl<'a> DebugPrint<'a> {
//     fn new(program: &'a [WithSpan<Stmt>]) -> Self {
//         Self {
//             result: Default::default(),
//             program,
//         }
//     }
//
//     fn line(&mut self, string: &str) {
//         self.result.push_str(string);
//         self.result.push('\n');
//     }
//
//     fn separator(&mut self) {
//         self.line("------------------------------");
//     }
//
//     fn source(&self, range: Span) -> &'a str {
//         let (start, end) = (range.start.0, range.end.0);
//         if start == end {
//             if start == 0 {
//                 &self.source[0..1]
//             } else {
//                 &self.source[(range.start.0 - 1)..(range.end.0)]
//             }
//         } else {
//             &self.source[(range.start.0)..(range.end.0)]
//         }
//     }
//
//     fn expr(&mut self, expr: &WithSpan<Expr>) {
//         match &expr.value {
//             Expr::Nil => todo!(),
//             Expr::Number(number) => todo!(),
//             Expr::Bool(_) => todo!(),
//             Expr::String(_) => todo!(),
//             Expr::Grouping(_) => todo!(),
//             Expr::Unary(_, _) => todo!(),
//             Expr::Binary(binary_expr) => todo!(),
//             Expr::Identifier(_) => todo!(),
//             Expr::Call(_, items) => todo!(),
//             Expr::If(if_expr) => todo!(),
//             Expr::Block(items) => todo!(),
//         }
//     }
//
//     fn generate(&mut self) {
//         for stmt in self.program {
//             match &stmt.value {
//                 Stmt::Expr(stmt_expr) => {
//                     for expr in &stmt_expr.exprs {
//                         self.expr(expr);
//                     }
//                     if let Some(semi) = stmt_expr.semi {
//                         self.line(&format!("semi: {}", self.source(semi)));
//                     }
//                 }
//                 Stmt::Item(item) => {}
//                 Stmt::Assign(items, items1) => {}
//                 Stmt::Binding(binding) => {
//                     for ident in &binding.identifiers {
//                         self.line(&format!("{}: {}", &ident.value, self.source(ident.span)));
//                     }
//                     if let Some(values) = &binding.values {
//                         for value in values {}
//                     }
//                 }
//                 Stmt::Print(_) => {}
//                 Stmt::Empty => {}
//             }
//             self.separator();
//         }
//     }
// }
//
// pub fn debug_print(program: &[WithSpan<Stmt>]) -> String {
//     let mut debug_print = DebugPrint::new(program);
//     debug_print.generate();
//     debug_print.result
// }
