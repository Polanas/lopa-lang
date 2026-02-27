use std::{collections::HashMap, ptr::fn_addr_eq};

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
pub struct Union {
    pub left: Box<Type>,
    pub right: Box<Type>,
}

#[derive(Debug, Clone)]
pub enum Type {
    Primitive(Primitive),
    Nilable(Box<Type>),
    Struct(TypeId),
    Fn(TypeId),
    BareFn(BareFn),
    Enum(TypeId),
    Array(Box<Type>),
    Block(Vec<Type>),
    Union(Union),
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
    pub name: Option<String>,
    pub ty: Type,
    pub default_value: Option<Type>,
}

#[derive(Debug, Clone)]
pub enum Fields {
    Unit,
    Named(Vec<Field>),
    Unnamed(Vec<Field>),
}

#[derive(Debug, Clone)]
pub enum Member {
    Fn(Fn),
}

impl Member {
    fn name(&self) -> &str {
        match self {
            Member::Fn(f) => &f.name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Struct {
    pub name: String,
    pub fields: Fields,
    pub members: Vec<Member>,
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
pub struct BareFn {
    pub output: Vec<Type>,
    pub params: Vec<FnParam>,
    pub variadic: Option<Box<Variadic>>,
}

#[derive(Debug, Clone)]
pub struct Fn {
    pub name: String,
    pub output: Vec<Type>,
    pub params: Vec<FnParam>,
    pub variadic: Option<Variadic>,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Fields,
    pub discriminant: Option<Type>,
}

#[derive(Debug, Clone)]
pub struct Enum {
    pub name: String,
    pub variants: Vec<Variant>,
    pub members: Vec<Member>,
}

#[derive(Debug, Clone)]
pub enum ComplexType {
    Struct(Struct),
    Fn(Fn),
    Enum(Enum),
}

impl ComplexType {
    fn kind(&self) -> ComplexTypeKind {
        match self {
            ComplexType::Struct(_) => ComplexTypeKind::Struct,
            ComplexType::Fn(_) => ComplexTypeKind::Fn,
            ComplexType::Enum(_) => ComplexTypeKind::Enum,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ComplexTypeKind {
    Struct,
    Fn,
    Enum,
}

impl ComplexType {
    fn name(&self) -> &str {
        match self {
            ComplexType::Struct(s) => &s.name,
            ComplexType::Fn(f) => &f.name,
            ComplexType::Enum(e) => &e.name,
        }
    }
}

#[derive(Debug, Default)]
struct Env {
    // fns, statics
    values_by_names: HashMap<String, (TypeId, ComplexTypeKind)>,
    // structs, nums, traits
    types_by_names: HashMap<String, (TypeId, ComplexTypeKind)>,
    types_by_ids: HashMap<AstNodeId, Type>,
    complex_types: HashMap<TypeId, ComplexType>,
    has_progress: bool,
    last_type_id: TypeId,
    diagnostics: Vec<position::Diagnostic>,
}

impl Env {
    fn new() -> Self {
        Self {
            types_by_ids: Default::default(),
            complex_types: Default::default(),
            has_progress: Default::default(),
            last_type_id: Default::default(),
            values_by_names: Default::default(),
            types_by_names: Default::default(),
            diagnostics: Default::default(),
        }
    }

    fn add_error(&mut self, message: &str, span: position::Span) {
        self.diagnostics.push(position::Diagnostic {
            span,
            message: message.to_owned(),
        });
    }

    fn type_id(&mut self) -> TypeId {
        let id = self.last_type_id;
        self.last_type_id += TypeId(1);
        id
    }

    fn add_value(&mut self, ty: ComplexType) -> TypeId {
        let id = self.type_id();
        self.values_by_names
            .insert(ty.name().to_owned(), (id, ty.kind()));
        self.complex_types.insert(id, ty);
        id
    }

    fn add_type(&mut self, ty: ComplexType) -> TypeId {
        let id = self.type_id();
        self.types_by_names
            .insert(ty.name().to_owned(), (id, ty.kind()));
        self.complex_types.insert(id, ty);
        id
    }

    fn type_expr(&mut self, type_expr: &ast::TypeExpr) -> Type {
        match type_expr {
            ast::TypeExpr::Array(type_expr) => {
                let inner = self.type_expr(type_expr);
                Type::Array(inner.into())
            }
            ast::TypeExpr::BareFn(bare_fn_type) => {
                bare_fn_type.output
                Type::BareFn(BareFn { output: (), params: (), variadic: () })
            }
            ast::TypeExpr::Nilable(type_expr) => Type::Nilable(self.type_expr(type_expr).into()),
            ast::TypeExpr::Path(path) => {
                //TODO: add proper modules support
                let name = &path.segments[0].ident.value;

                if let Some(value) = self.values_by_names.get(name.as_str()) {
                    // Type::
                }
                todo!()
            }
            ast::TypeExpr::SelfType(_) => Type::Blank,
            ast::TypeExpr::Primitive(primitive_type) => Type::Primitive(primitive_type.value),
            ast::TypeExpr::Paren(type_expr) => self.type_expr(type_expr),
            ast::TypeExpr::Tuple(_tuple_type) => todo!(),
            ast::TypeExpr::Union(union_type) => todo!(),
        }
    }

    fn strct(&mut self, strct: &ast::ItemStruct) {
        if self.types_by_names.contains_key(&strct.name.value) {
            self.add_error(
                &format!(
                    "The struct named '{}' is added multiple times",
                    &strct.name.value
                ),
                strct.span,
            );
        } else {
            let ty = match &strct.fields {
                ast::Fields::Unit => ComplexType::Struct(Struct {
                    name: strct.name.value.clone(),
                    fields: Fields::Unit,
                    members: Default::default(),
                }),
                ast::Fields::Named(fields_named) => {
                    let mut fields = Vec::new();
                    for field in &fields_named.fields {
                        let name = field.name.as_ref().unwrap().value.clone();
                        fields.push(Field {
                            ty: self.type_expr(&field.ty),
                            default_value: field.default_value.as_ref().map(|_| Type::Blank),
                            name: Some(name.clone()),
                        });
                    }
                    ComplexType::Struct(Struct {
                        name: strct.name.value.clone(),
                        fields: Fields::Named(fields),
                        members: Default::default(),
                    })
                }
                ast::Fields::Unnamed(fields_unnamed) => {
                    let mut fields = Vec::new();
                    for field in &fields_unnamed.fields {
                        fields.push(Field {
                            ty: self.type_expr(&field.ty),
                            default_value: field.default_value.as_ref().map(|_| Type::Blank),
                            name: None,
                        });
                    }
                    ComplexType::Struct(Struct {
                        name: strct.name.value.clone(),
                        fields: Fields::Named(fields),
                        members: Default::default(),
                    })
                }
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
        for item in program {
            self.item(item);
        }
        // loop {
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

    fn add_error(&mut self, message: &str, span: position::Span) {
        self.diagnostics.push(position::Diagnostic {
            span,
            message: message.to_owned(),
        });
    }

    pub fn set_source(&mut self, source: &'a str) {
        self.source = Some(source);
    }

    pub fn source(&self) -> &'a str {
        self.source.as_ref().unwrap()
    }

    pub fn check(&mut self, program: &[ast::Item]) {
        //TODO: check default values after collecting definitions
        self.env.collect(program);
        self.diagnostics.append(&mut self.env.diagnostics);
    }

    pub fn diagnostics(&self) -> &[position::Diagnostic] {
        &self.diagnostics
    }
}

impl<'a> Default for Context<'a> {
    fn default() -> Self {
        Self::new()
    }
}
