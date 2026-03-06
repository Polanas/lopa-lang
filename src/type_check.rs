use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use itertools::Itertools;

use crate::{
    ast::{self, AstNodeId},
    common::{self, BinaryOrAssignOp, Primitive},
    position::{self, Spanned, WithSpan},
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
    PartialOrd,
    Ord,
    Eq,
    Hash,
    derive_more::Add,
    derive_more::From,
    derive_more::AddAssign,
)]
pub struct TypeId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Union {
    pub types: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Type {
    Primitive(Primitive),
    Nilable(Box<Type>),
    Struct(TypeId),
    Fn(TypeId),
    Enum(TypeId),
    BareFn(BareFn),
    Array(Box<Type>),
    Block(Vec<Type>),
    Union(Union),
    Receiver,
    Blank,
}

impl Type {
    pub fn is_number(&self) -> bool {
        matches!(self, Type::Primitive(p) if p.is_number())
    }

    pub fn is_nilable(&self) -> bool {
        matches!(self, Self::Nilable(_))
    }

    pub fn unwrap_nil_ref(&self) -> &Self {
        match self {
            Self::Nilable(inner) => inner,
            other => other,
        }
    }

    pub fn unwrap_nil_mut(&mut self) -> &mut Self {
        match self {
            Self::Nilable(inner) => inner,
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
pub struct MemberFn {
    value: Fn,
    op: Option<BinaryOrAssignOp>,
}

#[derive(Debug, Clone)]
pub enum Member {
    Fn(MemberFn),
}

impl Member {
    pub fn name(&self) -> &str {
        match self {
            Member::Fn(f) => &f.value.name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Members {
    pub value: Vec<Member>,
}

impl Members {
    pub fn op_member(&self, op: BinaryOrAssignOp) -> Option<&Member> {
        for member in &self.value {
            match member {
                Member::Fn(member_fn) => {
                    if let Some(member_op) = member_fn.op
                        && member_op == op
                    {
                        return Some(member);
                    }
                }
            }
        }

        None
    }
}

#[derive(Debug, Clone)]
pub struct Struct {
    pub name: String,
    pub fields: Fields,
    pub members: Vec<Member>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReturnType {
    None,
    Type(Vec<Type>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BareFnParam {
    pub name: Option<String>,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
        if let Type::Receiver = ty {
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
    }

    fn resolve(mut self) {
        let mut complex_types = self.env.complex_types.clone();
        for (id, complex_type) in complex_types.iter_mut() {
            self.current_item = Some((
                *id,
                match complex_type {
                    ComplexType::Struct(_) => TypeItemKind::Struct,
                    ComplexType::Enum(_) => TypeItemKind::Enum,
                    _ => continue,
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
                    for param in func.value.params.iter_mut() {
                        if let FnParam::Typed(param) = param {
                            self.update_receiver(&mut param.ty);
                        }
                    }
                    match &mut func.value.output {
                        ReturnType::None => {}
                        ReturnType::Type(items) => {
                            for item in items.iter_mut() {
                                self.update_receiver(item);
                            }
                        }
                    }
                    if let Some(variadic) = &mut func.value.variadic {
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

struct DefsResolver<'env> {
    env: &'env mut Env,
}

impl<'env> Deref for DefsResolver<'env> {
    type Target = Env;

    fn deref(&self) -> &Self::Target {
        self.env
    }
}

impl<'env> DerefMut for DefsResolver<'env> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.env
    }
}

impl<'env> DefsResolver<'env> {
    fn new(env: &'env mut Env) -> Self {
        Self { env }
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

        let op = func.attribs.iter().find_map(|a| {
            if let ast::Attrib::Operator(op) = a {
                Some(op.op)
            } else {
                None
            }
        });
        let func = self.func(func);
        Some(Member::Fn(MemberFn {
            value: func,
            op: op.clone(),
        }))
    }

    fn struct_def(&mut self, strct: &ast::ItemStruct) -> Option<()> {
        if self.type_items.contains_key(&strct.name.value) {
            self.add_error(
                &format!(
                    "struct named '{}' is defined multiple times",
                    &strct.name.value
                ),
                strct.span,
            );
            return None;
        }
        let fields = self.fields_def(&strct.fields);
        let ty = ComplexType::Struct(Struct {
            name: strct.name.value.clone(),
            fields,
            members: Default::default(),
        });
        let id = self.add_type(ty);
        self.types_by_ids.insert(strct.id, Type::Struct(id));
        Some(())
    }

    fn fn_def(&mut self, func: &ast::ItemFn) -> Option<()> {
        if self.value_items.contains_key(&func.name.value) {
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
        let id = self.add_value(ComplexType::Fn(ty));
        self.types_by_ids.insert(func.id, Type::Fn(id));
        Some(())
    }

    fn enum_def(&mut self, en: &ast::ItemEnum) -> Option<()> {
        if self.type_items.contains_key(&en.name.value) {
            self.add_error(
                &format!("en named '{}' is defined multiple times", &en.name.value),
                en.span,
            );
            return None;
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
        Some(())
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

    fn inline_def(&mut self, item_inline: &ast::ItemInline) -> Option<()> {
        for def in &item_inline.defs {
            if self.value_items.contains_key(&def.name.value) {
                self.add_error(
                    &format!(
                        "function named '{}' is defined multiple times",
                        &def.name.value
                    ),
                    def.span,
                );
                return None;
            }
            let ty = self.inline_func(def);
            let id = self.add_type(ComplexType::Fn(ty));
            self.types_by_ids.insert(def.id, Type::Fn(id));
        }
        Some(())
    }

    fn item_def(&mut self, item: &ast::Item) -> Option<()> {
        match item {
            ast::Item::Struct(item_struct) => self.struct_def(item_struct),
            ast::Item::Fn(item_fn) => self.fn_def(item_fn),
            ast::Item::Enum(item_enum) => self.enum_def(item_enum),
            ast::Item::Inline(item_inline) => self.inline_def(item_inline),
            _ => Some(()),
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
            match self.type_item_mut(item_kind, target_id) {
                TypeItemMut::Struct(s) => s.members.extend(members),
                TypeItemMut::Enum(e) => e.members.extend(members),
            }
            Some(())
        } else {
            Some(())
        }
    }

    fn resolve(mut self, program: &[ast::Item]) -> Option<()> {
        for item in program {
            self.item_def(item)?;
        }
        for item in program {
            self.impl_def(item)?;
        }
        Some(())
    }
}

struct BlankResolver<'env> {
    has_progress: bool,
    env: &'env mut Env,
}

impl<'env> Deref for BlankResolver<'env> {
    type Target = Env;

    fn deref(&self) -> &Self::Target {
        self.env
    }
}

impl<'env> DerefMut for BlankResolver<'env> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.env
    }
}

impl<'env> BlankResolver<'env> {
    fn new(env: &'env mut Env) -> Self {
        Self {
            env,
            has_progress: false,
        }
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

    fn enum_item(&mut self, ast_enum: &ast::ItemEnum) -> Option<()> {
        let Type::Enum(id) = self.env.type_items.get_mut(&ast_enum.name.value).unwrap() else {
            unreachable!();
        };
        let mut complex_types = self.env.complex_types.clone();
        let ComplexType::Enum(en) = complex_types.get_mut(id).unwrap() else {
            unreachable!();
        };

        for (variant, ast_variant) in en.variants.iter_mut().zip(ast_enum.variants.iter()) {
            self.update_fields(&mut variant.fields, &ast_variant.fields);
        }

        Some(())
    }

    fn struct_item(&mut self, ast_strct: &ast::ItemStruct) -> Option<()> {
        let Type::Struct(id) = self.env.type_items.get_mut(&ast_strct.name.value).unwrap() else {
            unreachable!()
        };
        let mut complex_types = self.env.complex_types.clone();
        let ComplexType::Struct(strct) = complex_types.get_mut(id).unwrap() else {
            unreachable!();
        };

        self.update_fields(&mut strct.fields, &ast_strct.fields);
        Some(())
    }

    fn fn_item(&mut self, ast_func: &ast::ItemFn) -> Option<()> {
        let Type::Fn(id) = self.env.value_items.get_mut(&ast_func.name.value).unwrap() else {
            unreachable!()
        };
        let mut complex_types = self.env.complex_types.clone();
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
        Some(())
    }

    fn extern_item(&mut self, extern_item: &ast::ItemExtern) -> Option<()> {
        for def in &extern_item.defs {
            let ast::ExternDefinition::Fn(def) = def;
            let Type::Fn(id) = self.env.value_items.get_mut(&def.name.value).unwrap() else {
                unreachable!()
            };
            let mut complex_types = self.env.complex_types.clone();
            let ComplexType::Fn(func) = complex_types.get_mut(id).unwrap() else {
                unreachable!()
            };

            match &mut func.output {
                ReturnType::None => {}
                ReturnType::Type(items) => {
                    let ast::ReturnType::Type(ast_items) = &def.output else {
                        unreachable!()
                    };

                    for (ty, ast_ty) in items.iter_mut().zip(ast_items.iter()) {
                        self.update_type(ty, ast_ty);
                    }
                }
            }

            for (param, ast_param) in func.params.iter_mut().zip(def.params.iter()) {
                if let FnParam::Typed(typed) = param {
                    let ast::FnParam::Typed(ast_typed) = ast_param else {
                        unreachable!()
                    };

                    self.update_type(&mut typed.ty, &ast_typed.pat_type.ty);
                }
            }

            if let Some(variadic) = &mut func.variadic {
                let Some(ast_variadic) = &def.variadic else {
                    unreachable!()
                };

                self.update_type(&mut variadic.ty, &ast_variadic.ty);
            }
        }
        Some(())
    }

    fn inline_item(&mut self, inline_item: &ast::ItemInline) -> Option<()> {
        for def in &inline_item.defs {
            let Type::Fn(id) = self.env.value_items.get_mut(&def.name.value).unwrap() else {
                unreachable!()
            };
            let mut complex_types = self.env.complex_types.clone();
            let ComplexType::Fn(func) = complex_types.get_mut(id).unwrap() else {
                unreachable!()
            };

            match &mut func.output {
                ReturnType::None => {}
                ReturnType::Type(items) => {
                    let ast::ReturnType::Type(ast_items) = &def.output else {
                        unreachable!()
                    };

                    for (ty, ast_ty) in items.iter_mut().zip(ast_items.iter()) {
                        self.update_type(ty, ast_ty);
                    }
                }
            }

            for (param, ast_param) in func.params.iter_mut().zip(def.params.iter()) {
                if let FnParam::Typed(typed) = param {
                    let ast::FnParam::Typed(ast_typed) = ast_param else {
                        unreachable!()
                    };

                    self.update_type(&mut typed.ty, &ast_typed.pat_type.ty);
                }
            }

            if let Some(variadic) = &mut func.variadic {
                let Some(ast_variadic) = &def.variadic else {
                    unreachable!()
                };

                self.update_type(&mut variadic.ty, &ast_variadic.ty);
            }
        }
        Some(())
    }

    fn impl_item(&mut self, item_impl: &ast::ItemImpl) -> Option<()> {
        let target = self.type_expr(&item_impl.target);
        let target_id = match target {
            Type::Struct(type_id) => type_id,
            Type::Enum(type_id) => type_id,
            _ => {
                unreachable!()
            }
        };

        let mut complex_types = self.complex_types.clone();
        let members = match complex_types.get_mut(&target_id).unwrap() {
            ComplexType::Struct(s) => &mut s.members,
            ComplexType::Enum(e) => &mut e.members,
            _ => unreachable!(),
        };

        for ast::ImplItem::Fn(ast_func) in &item_impl.items {
            let Member::Fn(func) = members
                .iter_mut()
                .find(|m| m.name() == ast_func.name.value)
                .unwrap();

            match &mut func.value.output {
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

            for (param, ast_param) in func.value.params.iter_mut().zip(ast_func.params.iter()) {
                if let FnParam::Typed(typed) = param {
                    let ast::FnParam::Typed(ast_typed) = ast_param else {
                        unreachable!()
                    };

                    self.update_type(&mut typed.ty, &ast_typed.pat_type.ty);
                }
            }

            if let Some(variadic) = &mut func.value.variadic {
                let Some(ast_variadic) = &ast_func.variadic else {
                    unreachable!()
                };

                self.update_type(&mut variadic.ty, &ast_variadic.ty);
            }
        }

        Some(())
    }

    fn item(&mut self, item: &ast::Item) -> Option<()> {
        match item {
            ast::Item::Struct(item_struct) => self.struct_item(item_struct),
            ast::Item::Fn(item_fn) => self.fn_item(item_fn),
            ast::Item::Enum(item_enum) => self.enum_item(item_enum),
            ast::Item::Impl(item_impl) => self.impl_item(item_impl),
            ast::Item::Inline(item_inline) => self.inline_item(item_inline),
            ast::Item::Extern(item_extern) => Some(()),
        }
    }

    fn resolve(mut self, program: &[ast::Item]) -> Option<()> {
        loop {
            self.has_progress = false;
            for item in program {
                self.item(item);
            }

            if !self.has_progress {
                break Some(());
            }
        }
    }
}

enum ValueItemRef<'a> {
    Fn(&'a Fn),
}

enum TypeItemRef<'a> {
    Struct(&'a Struct),
    Enum(&'a Enum),
}
enum TypeItemMut<'a> {
    Struct(&'a mut Struct),
    Enum(&'a mut Enum),
}

#[derive(Debug)]
struct Env {
    // fns, statics
    value_items: HashMap<String, Type>,
    // structs, enums, traits
    type_items: HashMap<String, Type>,
    types_by_ids: HashMap<AstNodeId, Type>,
    complex_types: SharedMut<HashMap<TypeId, ComplexType>>,
    last_type_id: TypeId,
    diagnostics: Vec<position::Diagnostic>,
}

impl Env {
    fn new() -> Self {
        Self {
            types_by_ids: Default::default(),
            complex_types: SharedMut::new(Default::default()),
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

    fn value_item_ref<'a>(&'a self, kind: ValueItemKind, id: TypeId) -> ValueItemRef<'a> {
        match kind {
            ValueItemKind::Fn => {
                let ComplexType::Fn(func) = self.complex_types.get(&id).unwrap() else {
                    unreachable!()
                };
                ValueItemRef::Fn(func)
            }
            ValueItemKind::Static => todo!(),
        }
    }

    fn type_item_ref<'a>(&'a self, kind: TypeItemKind, id: TypeId) -> TypeItemRef<'a> {
        match kind {
            TypeItemKind::Struct => {
                let ComplexType::Struct(strct) = self.complex_types.get(&id).unwrap() else {
                    unreachable!()
                };
                TypeItemRef::Struct(strct)
            }
            TypeItemKind::Enum => {
                let ComplexType::Enum(en) = self.complex_types.get(&id).unwrap() else {
                    unreachable!()
                };
                TypeItemRef::Enum(en)
            }
            TypeItemKind::Trait => unimplemented!(),
        }
    }
    fn type_item_mut<'a>(&'a mut self, kind: TypeItemKind, id: TypeId) -> TypeItemMut<'a> {
        match kind {
            TypeItemKind::Struct => {
                let ComplexType::Struct(strct) = self.complex_types.get_mut(&id).unwrap() else {
                    unreachable!()
                };
                TypeItemMut::Struct(strct)
            }
            TypeItemKind::Enum => {
                let ComplexType::Enum(en) = self.complex_types.get_mut(&id).unwrap() else {
                    unreachable!()
                };
                TypeItemMut::Enum(en)
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
            match &ty {
                ComplexType::Fn(_) => Type::Fn(id),
                _ => unreachable!(),
            },
        );
        self.complex_types.insert(id, ty);
        id
    }

    fn add_type(&mut self, ty: ComplexType) -> TypeId {
        let id = self.type_id();
        self.type_items.insert(
            ty.name().to_owned(),
            match &ty {
                ComplexType::Struct(_) => Type::Struct(id),
                ComplexType::Enum(_) => Type::Enum(id),
                _ => unreachable!(),
            },
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

                if let Some(ty) = self.type_items.get(name.as_str()) {
                    return ty.clone();
                }
                if let Some(ty) = self.value_items.get(name.as_str()) {
                    return ty.clone();
                }

                Type::Blank
            }
            ast::TypeExpr::Receiver(_) => Type::Receiver,
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

    fn collect(&mut self, program: &[ast::Item]) -> Option<()> {
        DefsResolver::new(self).resolve(program)?;
        println!("resolving defs finished");
        ReceiverResolver::new(self).resolve();
        println!("resolving receiver finished");
        BlankResolver::new(self).resolve(program)?;
        println!("resolving blanks finished");
        Some(())
    }
}

trait GetType {
    fn get_type(&self) -> &Type;
}

macro_rules! impl_get_type {
    ($ident:ident) => {
        impl GetType for $ident {
            fn get_type(&self) -> &Type {
                &self.ty
            }
        }
    };
}

macro_rules! impl_get_type_enum {
    (
        $( #[$meta:meta] )*
        $vis:vis enum $name:ident {
            $($variant:ident $(($ty:ty))?,)*
        }
    ) => {
        $( #[$meta] )*
        $vis enum $name {
            $($variant $(($ty))?,)*
        }

        #[allow(non_snake_case)]
        impl GetType for $ident {
            fn get_type(&self) -> &Type {
                match self {
                    $($name::$variant (paste!{[<_ $name>]}) => {
                        paste!{[<_ $name>]}.get_type()
                    },)*
                }

            }
        }
    };
}

#[derive(Debug, Clone)]
pub struct PathExpr {
    pub ty: Type,
}
impl_get_type!(PathExpr);

#[derive(Debug, Clone)]
pub struct PrimitiveExpr {
    pub ty: Type,
}
impl_get_type!(PrimitiveExpr);

#[derive(Default, Debug)]
struct Scope {
    locals: HashMap<String, Type>,
}

#[derive(Default, Debug)]
struct Scopes {
    scopes: Vec<Scope>,
}

impl Scopes {
    fn push_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn scope(&self) -> &Scope {
        &self.scopes[self.scopes.len() - 1]
    }

    fn scope_mut(&mut self) -> &mut Scope {
        let len = self.scopes.len();
        &mut self.scopes[len - 1]
    }

    fn insert_local(&mut self, name: &str, ty: Type) {
        self.scope_mut().locals.insert(name.to_owned(), ty);
    }

    fn local(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.locals.get(name) {
                return Some(ty);
            }
        }
        None
    }
}

#[derive(Default, Debug, Clone)]
struct Context {
    current_fn: Option<TypeId>,
    current_impl_item: Option<Type>,
}

pub struct TypeCheck<'a> {
    diagnostics: Vec<position::Diagnostic>,
    env: Env,
    context: Context,
    scopes: Scopes,
    source: Option<&'a str>,
}

impl<'a> TypeCheck<'a> {
    pub fn new() -> Self {
        Self {
            diagnostics: Default::default(),
            source: Default::default(),
            env: Env::new(),
            scopes: Default::default(),
            context: Default::default(),
        }
    }

    pub fn set_source(&mut self, source: &'a str) {
        self.source = Some(source);
    }

    pub fn source(&self) -> &'a str {
        self.source.as_ref().unwrap()
    }

    fn add_error(&mut self, message: &str, span: position::Span) {
        self.diagnostics.push(position::Diagnostic {
            span,
            message: message.to_owned(),
        });
    }

    fn cast_fn_to_bare(&self, func: &Fn) -> BareFn {
        BareFn {
            output: func.output.clone(),
            params: func
                .params
                .iter()
                .map(|p| match p {
                    FnParam::Receiver => BareFnParam {
                        name: Some(String::from("self")),
                        ty: self.context.current_impl_item.clone().unwrap(),
                    },
                    FnParam::Typed(typed) => BareFnParam {
                        name: Some(typed.name.clone()),
                        ty: typed.ty.clone(),
                    },
                })
                .collect_vec(),
            variadic: func.variadic.clone().map(Into::into),
        }
    }

    //TODO: consider accounting for returning nothing vs nil?
    fn cmp_bare_fns(&self, left: &BareFn, right: &BareFn) -> bool {
        let variadics_cmp = match (&left.variadic, &right.variadic) {
            (Some(left), Some(right)) => left.ty == right.ty,
            (None, None) => true,
            _ => return false,
        };
        left.params == right.params && left.output == right.output && variadics_cmp
    }

    //is `left = right` possible?
    fn can_assing_type(&self, left: &mut Type, right: &mut Type) -> bool {
        match (left.is_nilable(), right.is_nilable()) {
            //T? = T?
            (true, true) => self.can_assing_type(left.unwrap_nil_mut(), right.unwrap_nil_mut()),
            //T? = T
            (true, false) => {
                matches!(right, Type::Primitive(Primitive::Nil))
                    || self.can_assing_type(left.unwrap_nil_mut(), right)
            }
            //T = T?
            (false, true) => false,
            //T = T
            (false, false) => {
                match (left, right) {
                    (Type::Primitive(Primitive::Nil), _) => true,
                    (Type::Primitive(left), Type::Primitive(right)) => match (left, right) {
                        //TODO: consider adding as casts to ensure safety and limit possiblities of
                        //hidden crashes
                        (Primitive::Any, _) | (_, Primitive::Any) => true,
                        (left, right) if left.is_number() && right.is_number() => true,
                        (left, right) => left == right,
                    },
                    (Type::Struct(left), Type::Struct(right)) => left == right,
                    (Type::Enum(left), Type::Enum(right)) => left == right,
                    (Type::BareFn(left), Type::BareFn(right)) => self.cmp_bare_fns(left, right),
                    (Type::BareFn(left), Type::Fn(right)) => {
                        let ValueItemRef::Fn(right) =
                            self.env.value_item_ref(ValueItemKind::Fn, *right);
                        self.cmp_bare_fns(left, &self.cast_fn_to_bare(right))
                    }
                    (Type::Fn(left), Type::Fn(right)) => {
                        let ValueItemRef::Fn(left) =
                            self.env.value_item_ref(ValueItemKind::Fn, *left);
                        let ValueItemRef::Fn(right) =
                            self.env.value_item_ref(ValueItemKind::Fn, *right);
                        self.cmp_bare_fns(&self.cast_fn_to_bare(left), &self.cast_fn_to_bare(right))
                    }
                    (Type::Array(left), Type::Array(right)) => self.can_assing_type(left, right),
                    (Type::Union(left), Type::Union(right)) => {
                        left.types.sort();
                        right.types.sort();
                        left.types
                            .iter_mut()
                            .zip(right.types.iter_mut())
                            .all(|(l, r)| self.can_assing_type(l, r))
                    }
                    (Type::Block(left), Type::Block(right)) => {
                        left.len() == right.len()
                            && left
                                .iter_mut()
                                .zip(right.iter_mut())
                                .all(|(l, r)| self.can_assing_type(l, r))
                    }
                    _ => false,
                }
            }
        }
    }

    fn display_type(&self, ty: &Type) -> String {
        match ty {
            Type::Primitive(primitive) => format!("{primitive}"),
            Type::Nilable(ty) => format!("{}?", self.display_type(ty)),
            Type::Struct(type_id) => {
                let TypeItemRef::Struct(strct) =
                    self.env.type_item_ref(TypeItemKind::Struct, *type_id)
                else {
                    unreachable!();
                };
                strct.name.clone()
            }
            Type::Fn(type_id) => {
                let ValueItemRef::Fn(func) = self.env.value_item_ref(ValueItemKind::Fn, *type_id);
                let params = func
                    .params
                    .iter()
                    .map(|p| match p {
                        FnParam::Receiver => unreachable!(),
                        FnParam::Typed(typed) => {
                            format!("{}: {}", &typed.name, self.display_type(&typed.ty))
                        }
                    })
                    .join(", ");
                let output = match &func.output {
                    ReturnType::None => None,
                    ReturnType::Type(types) => {
                        Some(types.iter().map(|t| self.display_type(t)).join(", "))
                    }
                };
                let returns_none = match &func.output {
                    ReturnType::None => true,
                    ReturnType::Type(type_exprs) => {
                        matches!(type_exprs.as_slice(), [Type::Primitive(Primitive::Nil)])
                    }
                };
                //TODO: what about inline / extern / etc?
                if let Some(output) = output
                    && !returns_none
                {
                    format!("fn {}({params}) -> {output}", &func.name)
                } else {
                    format!("fn {}({params})", &func.name)
                }
            }
            Type::Enum(type_id) => {
                let TypeItemRef::Enum(en) = self.env.type_item_ref(TypeItemKind::Enum, *type_id)
                else {
                    unreachable!();
                };
                en.name.clone()
            }
            Type::BareFn(bare_fn) => {
                let params = bare_fn
                    .params
                    .iter()
                    .map(|param| {
                        if let Some(name) = &param.name {
                            format!("{}: {}", &name, self.display_type(&param.ty))
                        } else {
                            self.display_type(&param.ty)
                        }
                    })
                    .join(", ");
                let output = match &bare_fn.output {
                    ReturnType::None => None,
                    ReturnType::Type(types) => {
                        Some(types.iter().map(|t| self.display_type(t)).join(", "))
                    }
                };
                let returns_none = match &bare_fn.output {
                    ReturnType::None => true,
                    ReturnType::Type(type_exprs) => {
                        matches!(type_exprs.as_slice(), [Type::Primitive(Primitive::Nil)])
                    }
                };
                if let Some(output) = output
                    && !returns_none
                {
                    format!("fn({params}) -> {output}")
                } else {
                    format!("fn({params})")
                }
            }
            Type::Array(ty) => format!("[{}]", self.display_type(ty)),
            Type::Block(items) => format!(
                "{{ {} }}",
                items.iter().map(|t| self.display_type(t)).join(", ")
            ),
            Type::Union(union) => union
                .types
                .iter()
                .map(|t| self.display_type(t))
                .join(" |")
                .to_string(),
            Type::Receiver => unreachable!(),
            Type::Blank => unreachable!(),
        }
    }

    fn source_substr(&self, range: position::Span) -> &str {
        let (start, end) = (range.start.0, range.end.0);
        if start == end {
            if start == 0 {
                &self.source()[0..1]
            } else {
                &self.source()[(range.start.0 - 1)..(range.end.0)]
            }
        } else {
            &self.source()[(range.start.0)..(range.end.0)]
        }
    }

    fn expr(&mut self, expr: &ast::Expr, mut expected: Option<&mut Type>) -> Option<Type> {
        let mut ty = match &expr {
            ast::Expr::Lit(lit_expr) => match lit_expr {
                ast::LitExpr::Nil(_) => Type::Primitive(Primitive::Nil),
                ast::LitExpr::Int(_) => Type::Primitive(Primitive::Int),
                ast::LitExpr::Float(_) => Type::Primitive(Primitive::Float),
                ast::LitExpr::Bool(_) => Type::Primitive(Primitive::Bool),
                ast::LitExpr::String(_) => Type::Primitive(Primitive::String),
            },
            ast::Expr::Array(array_expr) => {
                if let Some(expected) = expected.as_deref_mut() {
                    let Type::Array(expected) = expected else {
                        self.add_error(
                            &format!(
                                "could not assign {} to {}",
                                self.source_substr(expr.span()),
                                self.display_type(expected)
                            ),
                            expr.span(),
                        );
                        return None;
                    };
                    for elem in &array_expr.elements {
                        _ = self.expr(elem, Some(expected))?;
                    }
                    Type::Array(Box::new(*expected.clone()))
                } else {
                    todo!()
                }
            }
            ast::Expr::Ident(ident) => {
                if let Some(ty) = self.scopes.local(&ident.value) {
                    ty.clone()
                } else {
                    self.add_error(
                        &format!("cannot find value `{}` in this scope", &ident.value),
                        expr.span(),
                    );
                    return None;
                }
            }
            ast::Expr::Binary(binary_expr) => todo!(),
            ast::Expr::Block(block_expr) => todo!(),
            ast::Expr::Call(call_expr) => todo!(),
            ast::Expr::Closure(closure_expr) => todo!(),
            ast::Expr::For(for_expr) => todo!(),
            ast::Expr::FieldGet(field_get_expr) => todo!(),
            ast::Expr::Group(group_expr) => todo!(),
            ast::Expr::If(if_expr) => todo!(),
            ast::Expr::Index(index_expr) => todo!(),
            ast::Expr::Loop(loop_expr) => todo!(),
            ast::Expr::MethodCall(method_call_expr) => todo!(),
            ast::Expr::Struct(struct_expr) => todo!(),
            ast::Expr::Path(path) => todo!(),
            ast::Expr::Tuple(tuple_expr) => todo!(),
            ast::Expr::Unary(unary_expr) => todo!(),
            ast::Expr::While(while_expr) => todo!(),
        };
        if let Some(expected) = expected
            && !self.can_assing_type(expected, &mut ty)
        {
            self.add_assign_error(expected, &mut ty, expr.span());
            None
        } else {
            Some(ty)
        }
    }

    fn add_assign_error(&mut self, left: &Type, right: &mut Type, span: position::Span) {
        self.add_error(
            &format!(
                "could not assign {} to {}",
                self.display_type(right),
                self.display_type(left)
            ),
            span,
        );
    }

    fn fn_item(&mut self, item_fn: &ast::ItemFn) -> Option<()> {
        let Type::Fn(fn_id) = self.env.types_by_ids.get(&item_fn.id).unwrap() else {
            unreachable!()
        };
        let mut complex_types = self.env.complex_types.clone();
        let ComplexType::Fn(fn_type) = complex_types.get_mut(fn_id).unwrap() else {
            unreachable!()
        };
        self.context.current_fn = Some(*fn_id);
        self.scopes.push_scope();

        for (param, ast_param) in fn_type.params.iter_mut().zip(item_fn.params.iter()) {
            if let FnParam::Typed(param) = param {
                self.scopes.insert_local(&param.name, param.ty.clone());
                match ast_param {
                    ast::FnParam::Receiver(_) => {
                        self.add_error(
                            "self paramater is only allowed in associated functions (impl or trait functions)",
                            ast_param.span(),
                        );
                        return None;
                    }
                    ast::FnParam::Typed(ast_param) => {
                        if let Some(ast_default_value) = &ast_param.default_value {
                            param.default_value =
                                Some(self.expr(ast_default_value, Some(&mut param.ty))?);
                        }
                    }
                }
            }
        }

        self.scopes.pop_scope();
        self.context.current_fn = None;

        Some(())
    }

    fn impl_item(&mut self, item_impl: &ast::ItemImpl) -> Option<()> {
        Some(())
    }

    fn item(&mut self, item: &ast::Item) -> Option<()> {
        match item {
            ast::Item::Fn(item_fn) => self.fn_item(item_fn),
            ast::Item::Impl(item_impl) => self.impl_item(item_impl),
            _ => Some(()),
        }
    }

    fn push_global_scope(&mut self) {
        self.scopes.push_scope();
        for (name, ty) in &self.env.value_items {
            self.scopes.insert_local(name.as_str(), ty.clone());
        }
    }

    pub fn check(&mut self, program: &[ast::Item]) -> Option<()> {
        //TODO: check default values after collecting definitions
        self.env.collect(program)?;
        println!("collecting finished");

        self.push_global_scope();
        for item in program {
            self.item(item)?;
        }

        self.diagnostics.append(&mut self.env.diagnostics);
        Some(())
    }

    pub fn diagnostics(&self) -> &[position::Diagnostic] {
        &self.diagnostics
    }
}

impl<'a> Default for TypeCheck<'a> {
    fn default() -> Self {
        Self::new()
    }
}
