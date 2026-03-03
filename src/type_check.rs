use std::collections::HashMap;

use crate::{
    ast::{self, AstNodeId},
    common::{self, Primitive},
    position::{self, Spanned},
    shared_mut::SharedMut,
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
    pub types: Vec<Type>,
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
    Receiver,
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
    pub fn name(&self) -> &str {
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
pub enum FnParam {
    Receiver,
    Typed(FnParamTyped),
}

#[derive(Debug, Clone)]
pub struct FnParamTyped {
    pub name: String,
    pub ty: Type,
    pub default_value: Option<Type>,
}

#[derive(Debug, Clone)]
pub enum ReturnType {
    None,
    Type(Vec<Type>),
}

#[derive(Debug, Clone)]
pub struct BareFnParam {
    pub name: Option<String>,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct BareFn {
    pub output: ReturnType,
    pub params: Vec<BareFnParam>,
    pub variadic: Option<Box<Variadic>>,
}

#[derive(Debug, Clone)]
pub struct Fn {
    pub name: String,
    pub output: ReturnType,
    pub params: Vec<FnParam>,
    pub variadic: Option<Variadic>,
}

#[derive(Debug, Clone, Default)]
pub struct Discriminant {
    pub value: Option<isize>,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Fields,
    pub discriminant: Option<Discriminant>,
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

#[derive(Debug, Clone, Copy)]
pub enum TypeItemKind {
    Struct,
    Enum,
    Trait,
}

#[derive(Debug, Clone, Copy)]
pub enum ValueItemKind {
    Fn,
    Static,
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

struct ReceiverResolver<'env> {
    env: &'env mut Env,
    current_item: Option<(TypeId, TypeItemKind)>,
}

impl<'env> ReceiverResolver<'env> {
    fn new(env: &'env mut Env) -> Self {
        Self {
            env,
            current_item: None,
        }
    }

    fn update_receiver(&mut self, ty: &mut Type) {
        let (id, item_kind) = self.current_item.unwrap();
        match item_kind {
            TypeItemKind::Struct => {
                *ty = Type::Struct(id);
            }
            TypeItemKind::Enum => {
                *ty = Type::Enum(id);
            }
            _ => {}
        }
    }

    fn resolve(mut self) {
        let mut complex_types = self.env.complex_types.clone();
        for (id, complex_type) in complex_types.iter_mut() {
            self.current_item = Some((
                *id,
                match complex_type {
                    ComplexType::Struct(_) => TypeItemKind::Struct,
                    ComplexType::Enum(_) => TypeItemKind::Enum,
                    _ => unreachable!(),
                },
            ));

            match complex_type {
                ComplexType::Struct(strct) => {
                    self.resolve_fields(&mut strct.fields);
                    self.resolve_members(&mut strct.members);
                }
                ComplexType::Enum(en) => {
                    for variant in en.variants.iter_mut() {
                        self.resolve_fields(&mut variant.fields);
                    }
                    self.resolve_members(&mut en.members);
                }
                _ => {}
            }
        }
    }

    fn resolve_members(&mut self, members: &mut [Member]) {
        for member in members {
            match member {
                Member::Fn(func) => {
                    for param in func.params.iter_mut() {
                        if let FnParam::Typed(param) = param {
                            self.update_receiver(&mut param.ty);
                        }
                    }
                    match &mut func.output {
                        ReturnType::None => {}
                        ReturnType::Type(items) => {
                            for item in items.iter_mut() {
                                self.update_receiver(item);
                            }
                        }
                    }
                    if let Some(variadic) = &mut func.variadic {
                        self.update_receiver(&mut variadic.ty);
                    }
                }
            }
        }
    }

    fn resolve_fields(&mut self, fields: &mut Fields) {
        match fields {
            Fields::Unit => {}
            Fields::Named(fields) => {
                for field in fields.iter_mut() {
                    self.update_receiver(&mut field.ty);
                }
            }
            Fields::Unnamed(fields) => {
                for field in fields.iter_mut() {
                    self.update_receiver(&mut field.ty);
                }
            }
        }
    }
}

enum TypeItemRef<'a> {
    Struct(&'a mut Struct),
    Enum(&'a mut Enum),
}

#[derive(Debug)]
struct Env {
    // fns, statics
    value_items: HashMap<String, (TypeId, ValueItemKind)>,
    // structs, enums, traits
    type_items: HashMap<String, (TypeId, TypeItemKind)>,
    types_by_ids: HashMap<AstNodeId, Type>,
    complex_types: SharedMut<HashMap<TypeId, ComplexType>>,
    has_progress: bool,
    last_type_id: TypeId,
    diagnostics: Vec<position::Diagnostic>,
}

impl Env {
    fn new() -> Self {
        Self {
            types_by_ids: Default::default(),
            complex_types: SharedMut::new(Default::default()),
            has_progress: Default::default(),
            last_type_id: Default::default(),
            value_items: Default::default(),
            type_items: Default::default(),
            diagnostics: Default::default(),
        }
    }

    fn add_error(&mut self, message: &str, span: position::Span) {
        self.diagnostics.push(position::Diagnostic {
            span,
            message: message.to_owned(),
        });
    }

    fn type_item_ref(&mut self, kind: TypeItemKind, id: TypeId) -> TypeItemRef {
        match kind {
            TypeItemKind::Struct => {
                let ComplexType::Struct(strct) = self.complex_types.get_mut(&id).unwrap() else {
                    unreachable!()
                };
                TypeItemRef::Struct(strct)
            }
            TypeItemKind::Enum => {
                let ComplexType::Enum(en) = self.complex_types.get_mut(&id).unwrap() else {
                    unreachable!()
                };
                TypeItemRef::Enum(en)
            }
            TypeItemKind::Trait => unimplemented!(),
        }
    }

    fn type_id(&mut self) -> TypeId {
        let id = self.last_type_id;
        self.last_type_id += TypeId(1);
        id
    }

    fn add_value(&mut self, ty: ComplexType) -> TypeId {
        let id = self.type_id();
        self.value_items.insert(
            ty.name().to_owned(),
            (
                id,
                match ty {
                    ComplexType::Fn(_) => ValueItemKind::Fn,
                    _ => unreachable!(),
                },
            ),
        );
        self.complex_types.insert(id, ty);
        id
    }

    fn add_type(&mut self, ty: ComplexType) -> TypeId {
        let id = self.type_id();
        self.type_items.insert(
            ty.name().to_owned(),
            (
                id,
                match ty {
                    ComplexType::Struct(_) => TypeItemKind::Struct,
                    ComplexType::Enum(_) => TypeItemKind::Enum,
                    _ => unimplemented!(),
                },
            ),
        );
        self.complex_types.insert(id, ty);
        id
    }

    fn type_expr(&mut self, type_expr: &ast::TypeExpr) -> Type {
        match type_expr {
            ast::TypeExpr::Array(type_expr) => {
                let inner = self.type_expr(type_expr);
                Type::Array(inner.into())
            }
            ast::TypeExpr::BareFn(bare_fn) => Type::BareFn(BareFn {
                output: match &bare_fn.output {
                    ast::ReturnType::None => ReturnType::None,
                    ast::ReturnType::Type(type_exprs) => ReturnType::Type(
                        type_exprs
                            .iter()
                            .map(|ty| self.type_expr(ty))
                            .collect::<Vec<_>>(),
                    ),
                },
                params: bare_fn
                    .params
                    .iter()
                    .map(|p| match p {
                        ast::BareFnParam::Receiver(_) => BareFnParam {
                            name: None,
                            ty: Type::Receiver,
                        },
                        ast::BareFnParam::Typed(param) => BareFnParam {
                            name: param.ident.as_ref().map(|i| i.value.clone()),
                            ty: self.type_expr(&param.ty),
                        },
                    })
                    .collect::<_>(),
                variadic: bare_fn.variadic.as_ref().map(|v| {
                    Variadic {
                        name: v.ident.as_ref().map(|i| i.value.clone()),
                        ty: self.type_expr(&v.ty),
                    }
                    .into()
                }),
            }),
            ast::TypeExpr::Nilable(type_expr) => Type::Nilable(self.type_expr(type_expr).into()),
            ast::TypeExpr::Path(path) => {
                //TODO: add proper modules support
                let name = &path.segments[0].ident.value;

                if let Some((id, kind)) = self.type_items.get(name.as_str()) {
                    return match kind {
                        TypeItemKind::Struct => Type::Struct(*id),
                        TypeItemKind::Enum => Type::Struct(*id),
                        TypeItemKind::Trait => todo!(),
                    };
                }
                if let Some((id, kind)) = self.value_items.get(name.as_str()) {
                    return match kind {
                        ValueItemKind::Fn => Type::Fn(*id),
                        ValueItemKind::Static => todo!(),
                    };
                }

                Type::Blank
            }
            ast::TypeExpr::Receiver(_) => Type::Blank,
            ast::TypeExpr::Primitive(primitive_type) => Type::Primitive(primitive_type.value),
            ast::TypeExpr::Paren(type_expr) => self.type_expr(type_expr),
            ast::TypeExpr::Tuple(_tuple_type) => todo!(),
            ast::TypeExpr::Union(union_type) => {
                let mut types = vec![];
                let mut head = &*union_type.right;

                while let ast::TypeExpr::Union(ast::UnionType { left, right, .. }) = head {
                    types.push(self.type_expr(left));
                    head = right;
                }

                types.push(self.type_expr(head));
                Type::Union(Union { types })
            }
        }
    }

    fn en(&mut self, ast_enum: &ast::ItemEnum) {
        let (id, _) = self.type_items.get_mut(&ast_enum.name.value).unwrap();
        let mut complex_types = self.complex_types.clone();
        let ComplexType::Enum(en) = complex_types.get_mut(id).unwrap() else {
            unreachable!();
        };

        for (variant, ast_variant) in en.variants.iter_mut().zip(ast_enum.variants.iter()) {
            self.update_fields(&mut variant.fields, &ast_variant.fields);
        }
    }

    fn strct(&mut self, ast_strct: &ast::ItemStruct) {
        let (id, _) = self.type_items.get_mut(&ast_strct.name.value).unwrap();
        let mut complex_types = self.complex_types.clone();
        let ComplexType::Struct(strct) = complex_types.get_mut(id).unwrap() else {
            unreachable!();
        };

        self.update_fields(&mut strct.fields, &ast_strct.fields);
    }

    fn update_fields(&mut self, fields: &mut Fields, ast_fields: &ast::Fields) {
        match fields {
            Fields::Unit => {}
            Fields::Named(fields) => {
                let ast::Fields::Named(ast_fields) = &ast_fields else {
                    unreachable!()
                };
                for (field, ast_field) in fields.iter_mut().zip(ast_fields.fields.iter()) {
                    self.update_type(&mut field.ty, &ast_field.ty);
                }
            }
            Fields::Unnamed(fields) => {
                let ast::Fields::Unnamed(ast_fields) = &ast_fields else {
                    unreachable!()
                };
                for (field, ast_field) in fields.iter_mut().zip(ast_fields.fields.iter()) {
                    self.update_type(&mut field.ty, &ast_field.ty);
                }
            }
        }
    }

    fn update_type(&mut self, ty: &mut Type, expr: &ast::TypeExpr) {
        if let Type::Blank = ty {
            *ty = self.type_expr(expr);
            if !matches!(ty, Type::Blank) {
                self.has_progress = true;
            }
        }
    }

    fn strct_def(&mut self, strct: &ast::ItemStruct) {
        if self.type_items.contains_key(&strct.name.value) {
            self.add_error(
                &format!(
                    "struct named '{}' is defined multiple times",
                    &strct.name.value
                ),
                strct.span,
            );
            return;
        }
        let fields = self.fields_def(&strct.fields);
        let ty = ComplexType::Struct(Struct {
            name: strct.name.value.clone(),
            fields,
            members: Default::default(),
        });
        let id = self.add_type(ty);
        self.types_by_ids.insert(strct.id, Type::Struct(id));
    }

    fn fields_def(&mut self, fields: &ast::Fields) -> Fields {
        match &fields {
            ast::Fields::Unit => Fields::Unit,
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
                Fields::Named(fields)
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
                Fields::Unnamed(fields)
            }
        }
    }

    fn member_func_def(
        &mut self,
        func: &ast::ItemFn,
        target: TypeId,
        kind: TypeItemKind,
    ) -> Option<Member> {
        let mut complex_types = self.complex_types.clone();
        let members = match kind {
            TypeItemKind::Struct => {
                let ComplexType::Struct(strct) = complex_types.get_mut(&target).unwrap() else {
                    unreachable!()
                };
                &mut strct.members
            }
            TypeItemKind::Enum => {
                let ComplexType::Enum(en) = self.complex_types.get_mut(&target).unwrap() else {
                    unreachable!()
                };
                &mut en.members
            }
            TypeItemKind::Trait => unimplemented!(),
        };

        if members.iter().any(|m| m.name() == func.name.value) {
            self.add_error(
                &format!(
                    "function named '{}' is defined multiple times",
                    &func.name.value
                ),
                func.span,
            );
            return None;
        }

        let ty = self.func(func);
        Some(Member::Fn(ty))
    }

    fn func_def(&mut self, func: &ast::ItemFn) {
        if self.value_items.contains_key(&func.name.value) {
            self.add_error(
                &format!(
                    "function named '{}' is defined multiple times",
                    &func.name.value
                ),
                func.span,
            );
            return;
        }

        let ty = self.func(func);
        let id = self.add_type(ComplexType::Fn(ty));
        self.types_by_ids.insert(func.id, Type::Fn(id));
    }

    fn inline_func(&mut self, func: &ast::InlineFn) -> Fn {
        let output = match &func.output {
            ast::ReturnType::None => ReturnType::None,
            ast::ReturnType::Type(type_exprs) => {
                let mut types = vec![];
                for type_expr in type_exprs.iter() {
                    types.push(self.type_expr(type_expr));
                }
                ReturnType::Type(types)
            }
        };
        let mut params = vec![];
        for param in func.params.iter() {
            params.push(match param {
                ast::FnParam::Receiver(_) => FnParam::Receiver,
                ast::FnParam::Typed(param) => FnParam::Typed(FnParamTyped {
                    //TODO: check patterns accordingly
                    name: match &*param.pat_type.pat {
                        ast::Pat::Ident(pat_ident) => pat_ident.value.value.clone(),
                        ast::Pat::Paren(pat_paren) => unreachable!(),
                        ast::Pat::Path(path) => unreachable!(),
                    },
                    ty: self.type_expr(&param.pat_type.ty),
                    default_value: param.default_value.as_ref().map(|_| Type::Blank),
                }),
            });
        }
        Fn {
            name: func.name.value.clone(),
            output,
            params,
            variadic: func.variadic.as_ref().map(|v| Variadic {
                name: v.ident.as_ref().map(|i| i.value.clone()),
                ty: self.type_expr(&v.ty),
            }),
        }
    }

    fn func(&mut self, func: &ast::ItemFn) -> Fn {
        let output = match &func.output {
            ast::ReturnType::None => ReturnType::None,
            ast::ReturnType::Type(type_exprs) => {
                let mut types = vec![];
                for type_expr in type_exprs.iter() {
                    types.push(self.type_expr(type_expr));
                }
                ReturnType::Type(types)
            }
        };
        let mut params = vec![];
        for param in func.params.iter() {
            params.push(match param {
                ast::FnParam::Receiver(_) => FnParam::Receiver,
                ast::FnParam::Typed(param) => FnParam::Typed(FnParamTyped {
                    //TODO: check patterns accordingly
                    name: match &*param.pat_type.pat {
                        ast::Pat::Ident(pat_ident) => pat_ident.value.value.clone(),
                        ast::Pat::Paren(pat_paren) => unreachable!(),
                        ast::Pat::Path(path) => unreachable!(),
                    },
                    ty: self.type_expr(&param.pat_type.ty),
                    default_value: param.default_value.as_ref().map(|_| Type::Blank),
                }),
            });
        }
        Fn {
            name: func.name.value.clone(),
            output,
            params,
            variadic: func.variadic.as_ref().map(|v| Variadic {
                name: v.ident.as_ref().map(|i| i.value.clone()),
                ty: self.type_expr(&v.ty),
            }),
        }
    }

    fn global_func(&mut self, ast_func: &ast::ItemFn) {
        let (id, _) = self.value_items.get_mut(&ast_func.name.value).unwrap();
        let mut complex_types = self.complex_types.clone();
        let ComplexType::Fn(func) = complex_types.get_mut(id).unwrap() else {
            unreachable!()
        };

        match &mut func.output {
            ReturnType::None => {}
            ReturnType::Type(items) => {
                let ast::ReturnType::Type(ast_items) = &ast_func.output else {
                    unreachable!()
                };

                for (ty, ast_ty) in items.iter_mut().zip(ast_items.iter()) {
                    self.update_type(ty, ast_ty);
                }
            }
        }

        for (param, ast_param) in func.params.iter_mut().zip(ast_func.params.iter()) {
            if let FnParam::Typed(typed) = param {
                let ast::FnParam::Typed(ast_typed) = ast_param else {
                    unreachable!()
                };

                self.update_type(&mut typed.ty, &ast_typed.pat_type.ty);
            }
        }

        if let Some(variadic) = &mut func.variadic {
            let Some(ast_variadic) = &ast_func.variadic else {
                unreachable!()
            };

            self.update_type(&mut variadic.ty, &ast_variadic.ty);
        }
    }

    fn enum_def(&mut self, en: &ast::ItemEnum) {
        if self.type_items.contains_key(&en.name.value) {
            self.add_error(
                &format!("en named '{}' is defined multiple times", &en.name.value),
                en.span,
            );
            return;
        }
        let mut variants = vec![];
        for variant in en.variants.iter() {
            let fields = self.fields_def(&variant.fields);
            variants.push(Variant {
                name: variant.name.value.clone(),
                fields,
                discriminant: variant
                    .discriminant
                    .as_ref()
                    .map(|_| Discriminant::default()),
            })
        }
        let ty = ComplexType::Enum(Enum {
            name: en.name.value.clone(),
            variants,
            members: Default::default(),
        });
        let id = self.add_type(ty);
        self.types_by_ids.insert(en.id, Type::Enum(id));
    }

    fn impl_item(&mut self, item_impl: &ast::ItemImpl) {
        // let target = self.type_expr(&item_impl.target);
        // for item in &item_impl.items {
        //     match item {
        //         ast::ImplItem::Fn(item_fn) => {}
        //     }
        // }
        // match target {
        //     Type::Struct(type_id) => todo!(),
        //     Type::Enum(type_id) => todo!(),
        //     _ => {}
        // }
    }

    fn inline_def(&mut self, item_inline: &ast::ItemInline) {
        for def in &item_inline.defs {
            if self.value_items.contains_key(&def.name.value) {
                self.add_error(
                    &format!(
                        "function named '{}' is defined multiple times",
                        &def.name.value
                    ),
                    def.span,
                );
                return;
            }
        }
    }

    fn item_def(&mut self, item: &ast::Item) {
        match item {
            ast::Item::Struct(item_struct) => self.strct_def(item_struct),
            ast::Item::Fn(item_fn) => self.func_def(item_fn),
            ast::Item::Enum(item_enum) => self.enum_def(item_enum),
            ast::Item::Inline(item_inline) => self.inline_def(item_inline),
            _ => {}
        }
    }

    fn impl_def(&mut self, item: &ast::Item) -> Option<()> {
        if let ast::Item::Impl(item_impl) = item {
            let target = self.type_expr(&item_impl.target);
            let (target_id, item_kind) = match target {
                Type::Struct(type_id) => (type_id, TypeItemKind::Struct),
                Type::Enum(type_id) => (type_id, TypeItemKind::Enum),
                _ => {
                    self.add_error("cannot define impl for this type", item_impl.target.span());
                    return None;
                }
            };
            let members = item_impl
                .items
                .iter()
                .map(|item| match item {
                    ast::ImplItem::Fn(item_fn) => {
                        self.member_func_def(item_fn, target_id, item_kind)
                    }
                })
                .collect::<Option<Vec<_>>>()?;
            match self.type_item_ref(item_kind, target_id) {
                TypeItemRef::Struct(s) => s.members.extend(members),
                TypeItemRef::Enum(e) => e.members.extend(members),
            }
            Some(())
        } else {
            None
        }
    }

    fn item(&mut self, item: &ast::Item) {
        match item {
            ast::Item::Struct(item_struct) => self.strct(item_struct),
            ast::Item::Fn(item_fn) => {}
            ast::Item::Extern(item_extern) => {}
            ast::Item::Inline(item_inline) => {}
            ast::Item::Enum(item_enum) => self.en(item_enum),
            ast::Item::Impl(item_impl) => self.impl_item(item_impl),
        }
    }

    fn collect(&mut self, program: &[ast::Item]) {
        for item in program {
            self.item_def(item);
        }

        for item in program {
            self.impl_def(item);
        }

        loop {
            self.has_progress = false;
            for item in program {
                self.item(item);
            }

            if !self.has_progress {
                break;
            }
        }
        ReceiverResolver::new(self).resolve();
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
            env: Env::new(),
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
