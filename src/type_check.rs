use std::collections::HashMap;

use crate::{
    ast::{self, AstNodeId},
    common::{self, Primitive},
    position,
};

#[derive(Debug, PartialEq, Clone)]
pub struct Path {
    pub segments: Vec<String>,
}

#[derive(
    Debug,
    Clone,
    Default,
    Copy,
    PartialEq,
    Eq,
    Hash,
    derive_more::Add,
    derive_more::From,
    derive_more::AddAssign,
)]
pub struct TypeId(pub usize);

#[derive(Debug, Clone)]
pub enum Type {
    Primitive(Primitive),
    Nilable(Box<Type>),
    Struct(TypeId),
    Fn(TypeId),
    Array(Box<Type>),
    Block(Vec<Type>),
    Blank,
}

impl Type {
    pub fn is_nilable(&self) -> bool {
        matches!(self, Self::Nilable(_))
    }

    pub fn unwrap_nil(self) -> Self {
        match self {
            Self::Nilable(inner) => *inner,
            other => other,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Field {
    name: String,
    ty: Type,
    default_value: Option<Type>,
}

#[derive(Debug, Clone)]
pub struct Struct {
    pub name: String,
    pub fields: Option<HashMap<String, Field>>,
}

#[derive(Debug, Clone)]
pub struct Variadic {
    pub name: Option<String>,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct FnParam {
    pub param_kind: common::FnParamKind,
    pub name: Option<String>,
    pub ty: Type,
    pub default_value: Option<Type>,
}

#[derive(Debug, Clone)]
pub struct Fn {
    pub name: String,
    pub output: Vec<Type>,
    pub params: Vec<FnParam>,
    pub variadic: Option<Variadic>,
}

#[derive(Debug, Clone)]
pub enum ComplexType {
    Struct(Struct),
    Fn(Fn),
}

#[derive(Debug, Default)]
struct Env {
    types_by_names: HashMap<String, TypeId>,
    types_by_ids: HashMap<AstNodeId, Type>,
    types: HashMap<TypeId, ComplexType>,
    has_progress: bool,
    last_type_id: TypeId,
}

impl Env {
    fn new() -> Self {
        Self {
            types_by_ids: Default::default(),
            types: Default::default(),
            has_progress: Default::default(),
            last_type_id: Default::default(),
            types_by_names: Default::default(),
        }
    }

    fn type_id(&mut self) -> TypeId {
        let id = self.last_type_id;
        self.last_type_id += TypeId(1);
        id
    }

    fn add_type(&mut self, ty: ComplexType) -> TypeId {
        let id = self.type_id();
        let name = match &ty {
            ComplexType::Struct(s) => s.name.clone(),
            ComplexType::Fn(f) => f.name.clone(),
        };
        self.types_by_names.insert(name, id);
        self.types.insert(id, ty);
        id
    }

    fn stmt(&mut self, stmt: &ast::Stmt) {}

    fn expr(&mut self, expr: &ast::Expr) -> Type {
        todo!()
    }

    fn type_expr(&mut self, type_expr: &ast::TypeExpr) -> Type {
        match type_expr {
            ast::TypeExpr::Array(type_expr) => todo!(),
            ast::TypeExpr::BareFn(bare_fn_type) => todo!(),
            ast::TypeExpr::Nilable(type_expr) => todo!(),
            ast::TypeExpr::Path(path) => {
                //TODO: add proper modules support
                let name = &path.segments[0].ident.value;

                if self.types_by_names.contains_key(name.as_str()) {}
                todo!()
            }
            ast::TypeExpr::SelfType(self_type) => todo!(),
            ast::TypeExpr::Primitive(primitive_type) => todo!(),
            ast::TypeExpr::Paren(type_expr) => todo!(),
            ast::TypeExpr::Tuple(tuple_type) => todo!(),
        }
    }

    fn strct(&mut self, strct: &ast::ItemStruct) {
        if self.types_by_ids.insert(strct.id, Type::Blank).is_none() {
            self.has_progress = true;

            let ty = match &strct.fields {
                ast::Fields::Unit => ComplexType::Struct(Struct {
                    name: strct.name.value.clone(),
                    fields: None,
                }),
                ast::Fields::Named(fields_named) => {
                    let mut fields = HashMap::new();
                    for field in &fields_named.fields {
                        let name = field.name.as_ref().unwrap().value.clone();
                        fields.insert(
                            name.clone(),
                            Field {
                                name,
                                ty: self.type_expr(&field.ty),
                                default_value: field.default_value.as_ref().map(|d| self.expr(&d)),
                            },
                        );
                    }
                    ComplexType::Struct(Struct {
                        name: strct.name.value.clone(),
                        fields: Some(fields),
                    })
                }
                ast::Fields::Unnamed(fields_unnamed) => todo!(),
            };
            let id = self.add_type(ty);
            self.types_by_ids.insert(strct.id, Type::Struct(id));
        }
    }

    fn item(&mut self, item: &ast::Item) {
        match item {
            ast::Item::Struct(item_struct) => {
                self.strct(item_struct);
            }
            ast::Item::Fn(item_fn) => todo!(),
            ast::Item::Extern(item_extern) => todo!(),
            ast::Item::Inline(item_inline) => todo!(),
            ast::Item::Enum(item_enum) => todo!(),
            ast::Item::Impl(item_impl) => todo!(),
        }
    }

    fn collect(&mut self, program: &[ast::Item]) {
        // loop {
        for item in program {
            self.item(item);
        }
        // }
    }
}

pub struct Context<'a> {
    diagnostics: Vec<position::Diagnostic>,
    env: Env,
    source: Option<&'a str>,
}

impl<'a> Context<'a> {
    pub fn new() -> Self {
        Self {
            diagnostics: Default::default(),
            source: Default::default(),
            env: Default::default(),
        }
    }

    pub fn set_source(&mut self, source: &'a str) {
        self.source = Some(source);
    }

    pub fn source(&self) -> &'a str {
        self.source.as_ref().unwrap()
    }

    pub fn check(&mut self, program: &[ast::Item]) {
        self.env.collect(program);
    }
}

impl<'a> Default for Context<'a> {
    fn default() -> Self {
        Self::new()
    }
}
