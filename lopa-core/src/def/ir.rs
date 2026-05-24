use la_arena::{Idx, RawIdx};
use ustr::Ustr;

use crate::{
    common::LitKind,
    def::lower::{self, lower_type_expr},
    ide,
    parsing::ast::{self, BinaryOpKind, UnaryOpKind},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Local<'db> {
    pub parent: Function<'db>,
    pub pattern_id: PatternId,
}

#[derive(salsa::Supertype, Clone, PartialEq, Eq, Hash, Debug, salsa::Update)]
pub enum ModuleDef<'db> {
    Function(Function<'db>),
    Struct(Struct<'db>),
}

#[salsa::tracked(debug)]
pub struct Function<'db> {
    pub name: Ustr,
    pub ast_ptr: ast::AstPtr<ast::FnItem>,
    pub file: ide::File,
}

#[salsa::tracked(debug)]
pub struct Param<'db> {
    pub name: Option<Ustr>,
    pub ty: TypeExpr<'db>,
}

#[salsa::tracked]
impl<'db> Function<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn params(self, db: &'db dyn salsa::Database) -> Vec<Param<'db>> {
        let mut params = vec![];
        let file = self.file(db);
        let root = ide::parse(db, file).syntax_node(db);
        for param in self
            .ast_ptr(db)
            .to_node(&root)
            .params()
            .into_iter()
            .flat_map(|p| p.params())
        {
            let name = param.pattern().and_then(|p| {
                Some(match p {
                    ast::Pattern::NamePattern(name_patern) => name_patern,
                })
                .and_then(|n| n.name())
                .and_then(|n| n.text())
            });
            let ty = param
                .ty()
                .map(|ty| lower_type_expr(db, file, ty))
                .unwrap_or_else(|| TypeExpr::Unknown);
            params.push(Param::new(db, name, ty));
        }
        params
    }

    #[salsa::tracked(returns(ref))]
    pub fn output(self, db: &'db dyn salsa::Database) -> Option<TypeExpr<'db>> {
        let file = self.file(db);
        let root = ide::parse(db, file).syntax_node(db);
        let output = self.ast_ptr(db).to_node(&root).output()?.ty();
        output.map(|o| lower_type_expr(db, file, o))
    }
}

#[salsa::tracked(debug)]
pub struct Struct<'db> {
    pub name: Ustr,
    pub ast_ptr: ast::AstPtr<ast::StructItem>,
    pub file: ide::File,
}

#[salsa::tracked(debug)]
pub struct Field<'db> {
    pub name: Ustr,
    pub ty: TypeExpr<'db>,
}

#[salsa::tracked]
impl<'db> Struct<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn fields(self, db: &'db dyn salsa::Database) -> Vec<Field<'db>> {
        let mut fields = vec![];
        let file = self.file(db);
        let root = ide::parse(db, file).syntax_node(db);
        for field in self
            .ast_ptr(db)
            .to_node(&root)
            .fields()
            .into_iter()
            .flat_map(|p| p.fields())
        {
            let Some(name) = field.name().and_then(|n| n.text()) else {
                continue;
            };

            let ty = field
                .ty()
                .map(|ty| lower_type_expr(db, file, ty))
                .unwrap_or_else(|| TypeExpr::Unknown);
            fields.push(Field::new(db, name, ty));
        }

        fields
    }
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub enum TypeExpr<'db> {
    Unknown,
    Unit,
    Any,
    Lit(LitKind),
    Struct(Struct<'db>),
    Function(Function<'db>),
    Nilable(Box<TypeExpr<'db>>),
    BareFunction {
        params: Vec<Param<'db>>,
        output: Option<Box<TypeExpr<'db>>>,
    },
}

pub type ExprId = RawIdx;

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update)]
pub enum Expr<'db> {
    Missing,
    Unit,
    Path(Vec<Ustr>),
    Lit(LitKind),
    BlockExpr {
        stmts: Vec<Stmt<'db>>,
    },
    If {
        if_cond: ExprId,
        if_branch: Vec<Stmt<'db>>,
        else_branch: Option<ElseBranch<'db>>,
    },
    Unary {
        expr: ExprId,
        kind: UnaryOpKind,
    },
    Binary {
        left: ExprId,
        right: ExprId,
        kind: BinaryOpKind,
    },
    Return {
        expr: ExprId,
    },
    Index {
        base: ExprId,
        index: ExprId,
    },
    Call {
        func: ExprId,
        args: Vec<Arg>,
    },
    Paren {
        expr: ExprId,
    },
    Field {
        name: Ustr,
        expr: ExprId,
    },
    Method {
        name: Ustr,
        expr: ExprId,
        args: Vec<Arg>,
    },
    Record {
        path: Vec<Ustr>,
        fields: Vec<RecordField>,
    },
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update)]
pub struct RecordField {
    pub name: Ustr,
    pub expr: ExprId,
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update)]
pub enum ElseBranch<'db> {
    Else { stmts: Vec<Stmt<'db>> },
    ElseIf { expr: ExprId },
}

pub type PatternId = Idx<Pattern>;

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update)]
pub enum Pattern {
    Missing,
    Name(Ustr),
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Arg {
    Labeled { label: Ustr, value: ExprId },
    NonLabeled { value: ExprId },
}

impl Arg {
    pub fn value(&self) -> ExprId {
        match self {
            Arg::Labeled { value, .. } | Arg::NonLabeled { value } => *value,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update)]
pub enum Stmt<'db> {
    Let {
        pattern: PatternId,
        ty: Option<TypeExpr<'db>>,
        expr: ExprId,
    },
    Expr {
        expr: ExprId,
        semi: Option<()>,
    },
}
