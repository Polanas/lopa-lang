use crate::ast;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct BytePos(pub usize);

impl BytePos {
    pub fn shift(&mut self, ch: char) {
        self.0 += ch.len_utf8();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Span {
    pub start: BytePos,
    pub end: BytePos,
}

impl Span {
    pub fn new(start: BytePos, end: BytePos) -> Self {
        Self { start, end }
    }

    pub const fn empty() -> Self {
        Self {
            start: BytePos(0),
            end: BytePos(0),
        }
    }

    pub fn union(&self, other: Self) -> Self {
        Self {
            start: other.start.min(self.start),
            end: other.end.max(self.end),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct WithSpan<T> {
    pub value: T,
    pub span: Span,
}

impl<T> WithSpan<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    pub const fn empty(value: T) -> Self {
        Self {
            value,
            span: Span::empty(),
        }
    }

    pub fn union(&self, other: &Self) -> Span {
        self.span.union(other.span)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
}

pub struct LineOffsets {
    offsets: Vec<usize>,
    len: usize,
}

impl LineOffsets {
    pub fn new(data: &str) -> Self {
        let mut offsets = vec![0];

        for (i, val) in data.bytes().enumerate() {
            if val == b'\n' {
                offsets.push((i + 1) as _);
            }
        }

        Self {
            offsets,
            len: data.len(),
        }
    }

    pub fn line(&self, offset: BytePos) -> usize {
        let offset = offset.0;
        assert!(offset <= self.len);
        match self.offsets.binary_search(&offset) {
            Ok(line) => line,
            Err(line) => line,
        }
    }
}

// struct SpanDebugPrint<'a> {
//     result: String,
//     program: &'a [WithSpan<ast::Stmt>],
//     source: &'a str,
//     depth: usize,
// }
//
// impl<'a> SpanDebugPrint<'a> {
//     fn new(program: &'a [WithSpan<ast::Stmt>], source: &'a str) -> Self {
//         Self {
//             result: Default::default(),
//             program,
//             source,
//             depth: 0,
//         }
//     }
//
//     fn push(&mut self) {
//         self.depth += 1;
//     }
//     fn pop(&mut self) {
//         self.depth -= 1;
//     }
//
//     fn line(&mut self, line: &str) {
//         for _ in 0..self.depth {
//             self.result.push('\n');
//         }
//         self.result.push_str(&format!("{line}\n"));
//     }
//
//     fn named_line(&mut self, name: &str, line: &str) {
//         self.result.push_str(&format!("{name}: {line}\n"));
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
//     fn expr(&mut self, expr: &WithSpan<ast::Expr>) {
//         match &expr.value {
//             ast::Expr::Nil => self.named_line("nil", self.source(expr.span)),
//             ast::Expr::Number(number) => {
//                 self.named_line(&number.to_string(), self.source(expr.span))
//             }
//             ast::Expr::Bool(b) => self.named_line(&b.to_string(), self.source(expr.span)),
//             ast::Expr::String(s) => self.named_line(&format!("\"{s}\""), self.source(expr.span)),
//             ast::Expr::Grouping(e) => self.expr(e),
//             ast::Expr::Unary(op, e) => {
//                 self.named_line("unary op", self.source(op.span));
//                 self.expr(e);
//             }
//             ast::Expr::Binary(binary_expr) => {
//                 self.expr(&binary_expr.left);
//                 self.named_line("binary op", self.source(binary_expr.op.span));
//                 self.expr(&binary_expr.right);
//             }
//             ast::Expr::Identifier(s) => self.named_line(&s.to_string(), self.source(expr.span)),
//             ast::Expr::If(if_expr) => {
//                 self.expr(&if_expr.condition);
//                 self.block(&if_expr.then_branch);
//                 if let Some(e) = &if_expr.else_branch {
//                     self.expr(e);
//                 }
//             }
//             ast::Expr::Block(stmts) => self.block(stmts),
//             ast::Expr::Call(_, items) => todo!(),
//         };
//     }
//
//     fn block(&mut self, stmts: &[WithSpan<ast::Stmt>]) {
//         self.push();
//         for stmt in stmts {
//             self.stmt(stmt);
//         }
//         self.pop();
//     }
//
//     fn stmt(&mut self, stmt: &WithSpan<ast::Stmt>) {
//         match &stmt.value {
//             ast::Stmt::Expr(stmt_expr) => {
//                 for expr in &stmt_expr.exprs {
//                     self.expr(expr);
//                 }
//                 if let Some(semi) = stmt_expr.semi {
//                     self.line(&format!("semi: {}", self.source(semi)));
//                 }
//             }
//             ast::Stmt::Item(item) => {}
//             ast::Stmt::Assign(idents, values) => {
//                 for ident in idents {
//                     self.line(&format!("{}: {}", &ident.value, self.source(ident.span)));
//                 }
//                 for value in values {
//                     self.expr(value);
//                 }
//             }
//             ast::Stmt::Binding(binding) => {
//                 for ident in &binding.identifiers {
//                     self.line(&format!("{}: {}", &ident.value, self.source(ident.span)));
//                 }
//                 if let Some(values) = &binding.values {
//                     for value in values {
//                         self.expr(value);
//                     }
//                 }
//             }
//             ast::Stmt::Print(_) => {}
//             ast::Stmt::Empty => {}
//         }
//     }
//
//     fn generate(&mut self) {
//         for stmt in self.program {
//             self.stmt(stmt);
//             self.separator();
//         }
//     }
// }
//
// pub fn debug_print(program: &[WithSpan<ast::Stmt>], source: &str) -> String {
//     let mut debug_print = SpanDebugPrint::new(program, source);
//     debug_print.generate();
//     debug_print.result
// }
