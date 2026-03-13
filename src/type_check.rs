use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use itertools::{Itertools, Position};

use crate::{
    ast::{self, AstNodeId},
    common::{self, BinaryOp, Primitive},
    position::{self, Diagnostic, Spanned, WithSpan},
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
pub struct Tuple {
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
    Union(Union),
    Tuple(Tuple),
    Uninit(Box<Type>),
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

    pub fn is_initialised(&self) -> bool {
        !matches!(self, Self::Uninit(_))
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

    fn collapse_nil_inner(&mut self) {
        if let Type::Nilable(inner) = self {
            inner.collapse_nil();

            if inner.is_nilable()
                && let Type::Nilable(deep_inner) = std::mem::replace(&mut **inner, Type::Blank)
            {
                *self = Type::Nilable(deep_inner)
            }
        }
    }
    pub fn collapse_nil(&mut self) {
        self.collapse_nil_inner();
        if let Type::Nilable(inner) = self
            && **inner == Type::Primitive(Primitive::Nil)
        {
            *self = Type::Primitive(Primitive::Nil);
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
    op: Option<BinaryOp>,
}

#[derive(Debug, Clone)]
pub enum Member {
    Fn(MemberFn),
    Static(Static),
}

impl Member {
    pub fn name(&self) -> &str {
        match self {
            Member::Fn(f) => &f.value.name,
            Member::Static(s) => &s.name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Members {
    pub value: Vec<Member>,
}

impl Members {
    pub fn op_member(&self, op: BinaryOp) -> Option<&Member> {
        for member in &self.value {
            match member {
                Member::Fn(member_fn) => {
                    if let Some(member_op) = member_fn.op
                        && member_op == op
                    {
                        return Some(member);
                    }
                }
                Member::Static(_) => return None,
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
pub struct Static {
    pub name: String,
    //None indicates that there wasn't a type specified in the AST, meaning it will be resolved
    //later, during final type check phase
    pub ty: Option<Type>,
}

#[derive(Debug, Clone)]
pub enum ComplexType {
    Struct(Struct),
    Fn(Fn),
    Enum(Enum),
    Static(Static),
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
            ComplexType::Static(s) => &s.name,
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
                Member::Static(s) => {
                    // self.update_receiver(&mut s.ty);
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

    fn static_item(&mut self, static_item: &ast::ItemStatic) -> Static {
        Static {
            name: static_item.ident.value.clone(),
            ty: static_item.ty.as_ref().map(|t| self.type_expr(t)),
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
                        ast::Pat::Tuple(pat_tuple) => todo!(),
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

    fn member_static_def(
        &mut self,
        item_static: &ast::ItemStatic,
        target: TypeId,
        kind: TypeItemKind,
    ) -> Option<Member> {
        let members = self.members(target, kind);
        if members.iter().any(|m| m.name() == item_static.ident.value) {
            self.add_error(
                &format!(
                    "member named '{}' is defined multiple times",
                    &item_static.ident.value
                ),
                item_static.span,
            );
            return None;
        }
        Some(Member::Static(self.static_item(item_static)))
    }

    fn member_func_def(
        &mut self,
        func: &ast::ItemFn,
        target: TypeId,
        kind: TypeItemKind,
    ) -> Option<Member> {
        let members = self.members(target, kind);
        if members.iter().any(|m| m.name() == func.name.value) {
            self.add_error(
                &format!(
                    "member named '{}' is defined multiple times",
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

    fn members(&mut self, target: TypeId, kind: TypeItemKind) -> &mut Vec<Member> {
        let members = match kind {
            TypeItemKind::Struct => {
                let ComplexType::Struct(strct) = self.complex_types.get_mut(&target).unwrap()
                else {
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
        members
    }

    fn struct_def(&mut self, strct: &ast::ItemStruct) -> Option<()> {
        if self.type_items.contains_key(&strct.name.value) {
            self.add_error(
                &format!(
                    "item named '{}' is defined multiple times",
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
        let (id, _) = self.add_type(ty);
        self.types_by_ids.insert(strct.id, id);
        Some(())
    }

    fn static_def(&mut self, static_item: &ast::ItemStatic) -> Option<()> {
        if self.value_items.contains_key(&static_item.ident.value) {
            self.add_error(
                &format!(
                    "item named '{}' is defined multiple times",
                    &static_item.ident.value
                ),
                static_item.span,
            );
            return None;
        }
        let ty = self.static_item(static_item);
        let (id, _) = self.add_value(ComplexType::Static(ty));
        self.types_by_ids.insert(static_item.id, id);
        Some(())
    }

    fn fn_def(&mut self, func: &ast::ItemFn) -> Option<()> {
        if self.value_items.contains_key(&func.name.value) {
            self.add_error(
                &format!(
                    "item named '{}' is defined multiple times",
                    &func.name.value
                ),
                func.span,
            );
            return None;
        }

        let ty = self.func(func);
        let (id, _) = self.add_value(ComplexType::Fn(ty));
        self.types_by_ids.insert(func.id, id);
        Some(())
    }

    fn enum_def(&mut self, en: &ast::ItemEnum) -> Option<()> {
        if self.type_items.contains_key(&en.name.value) {
            self.add_error(
                &format!("item named '{}' is defined multiple times", &en.name.value),
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
        let (id, _) = self.add_type(ty);
        self.types_by_ids.insert(en.id, id);
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
                        ast::Pat::Tuple(pat_tuple) => todo!(),
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
                    &format!("item named '{}' is defined multiple times", &def.name.value),
                    def.span,
                );
                return None;
            }
            let ty = self.inline_func(def);
            let (id, _) = self.add_type(ComplexType::Fn(ty));
            self.types_by_ids.insert(def.id, id);
        }
        Some(())
    }

    fn item_def(&mut self, item: &ast::Item) -> Option<()> {
        match item {
            ast::Item::Struct(item_struct) => self.struct_def(item_struct),
            ast::Item::Fn(item_fn) => self.fn_def(item_fn),
            ast::Item::Enum(item_enum) => self.enum_def(item_enum),
            ast::Item::Inline(item_inline) => self.inline_def(item_inline),
            ast::Item::Static(item_static) => self.static_def(item_static),
            _ => todo!(),
            // ast::Item::Extern(item_extern) => todo!(),
            // ast::Item::Impl(item_impl) => todo!(),
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
                    ast::ImplItem::Static(item_static) => {
                        self.member_static_def(item_static, target_id, item_kind)
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
        let (id, _) = self.env.type_items.get_mut(&ast_enum.name.value).unwrap();
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
        let (id, _) = self.env.type_items.get_mut(&ast_strct.name.value).unwrap();
        let mut complex_types = self.env.complex_types.clone();
        let ComplexType::Struct(strct) = complex_types.get_mut(id).unwrap() else {
            unreachable!();
        };

        self.update_fields(&mut strct.fields, &ast_strct.fields);
        Some(())
    }

    fn fn_item(&mut self, ast_func: &ast::ItemFn) -> Option<()> {
        let (id, _) = self.env.value_items.get_mut(&ast_func.name.value).unwrap();
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
            let (id, _) = self.env.value_items.get_mut(&def.name.value).unwrap();
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

    fn static_item(&mut self, static_item: &ast::ItemStatic) -> Option<()> {
        let Some((id, _)) = self.value_items.get(&static_item.ident.value) else {
            unreachable!()
        };
        let mut complex_types = self.env.complex_types.clone();
        let Some(ComplexType::Static(static_ty)) = complex_types.get_mut(id) else {
            unreachable!()
        };
        if let Some(ty) = static_ty.ty.as_mut() {
            let ast_ty = static_item.ty.as_ref().unwrap();
            self.update_type(ty, ast_ty);
        }
        Some(())
    }

    fn inline_item(&mut self, inline_item: &ast::ItemInline) -> Option<()> {
        for def in &inline_item.defs {
            let (id, _) = self.env.value_items.get_mut(&def.name.value).unwrap();
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

        for item in &item_impl.items {
            match item {
                ast::ImplItem::Fn(ast_func) => {
                    let Member::Fn(func) = members
                        .iter_mut()
                        .find(|m| m.name() == ast_func.name.value)
                        .unwrap()
                    else {
                        unreachable!()
                    };

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

                    for (param, ast_param) in
                        func.value.params.iter_mut().zip(ast_func.params.iter())
                    {
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
                ast::ImplItem::Static(ast_static) => {
                    let Member::Static(item_static) = members
                        .iter_mut()
                        .find(|m| m.name() == ast_static.ident.value)
                        .unwrap()
                    else {
                        unreachable!()
                    };
                    if let Some(ty) = item_static.ty.as_mut() {
                        let ast_ty = ast_static.ty.as_ref().unwrap();
                        self.update_type(ty, ast_ty);
                    }
                }
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
            ast::Item::Static(item_static) => self.static_item(item_static),
            ast::Item::Extern(item_extern) => self.extern_item(item_extern),
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
    Static(&'a Static),
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
    value_items: HashMap<String, (TypeId, ValueItemKind)>,
    // structs, enums, traits
    type_items: HashMap<String, (TypeId, TypeItemKind)>,
    types_by_ids: HashMap<AstNodeId, TypeId>,
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
            ValueItemKind::Static => {
                let ComplexType::Static(static_item) = self.complex_types.get(&id).unwrap() else {
                    unreachable!()
                };
                ValueItemRef::Static(static_item)
            }
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

    fn add_value(&mut self, ty: ComplexType) -> (TypeId, ValueItemKind) {
        let id = self.type_id();
        let (id, kind) = match &ty {
            ComplexType::Fn(_) => (id, ValueItemKind::Fn),
            ComplexType::Static(_) => (id, ValueItemKind::Static),
            _ => unreachable!(),
        };
        self.value_items.insert(ty.name().to_owned(), (id, kind));
        self.complex_types.insert(id, ty);
        (id, kind)
    }

    fn add_type(&mut self, ty: ComplexType) -> (TypeId, TypeItemKind) {
        let id = self.type_id();
        let (id, kind) = match &ty {
            ComplexType::Struct(_) => (id, TypeItemKind::Struct),
            ComplexType::Enum(_) => (id, TypeItemKind::Struct),
            _ => unreachable!(),
        };
        self.type_items.insert(ty.name().to_owned(), (id, kind));
        self.complex_types.insert(id, ty);
        (id, kind)
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
                        TypeItemKind::Enum => Type::Enum(*id),
                        TypeItemKind::Trait => todo!(),
                    };
                }
                if let Some((id, kind)) = self.value_items.get(name.as_str()) {
                    return match kind {
                        ValueItemKind::Fn => Type::Fn(*id),
                        ValueItemKind::Static => {
                            //TODO: say 'expected type found static' and return None
                            todo!()
                            // let ComplexType::Static(static_ty) =
                            //     self.complex_types.get(&id).unwrap()
                            // else {
                            //     unreachable!()
                            // };
                            // static_ty.ty.clone().unwrap()
                        }
                    };
                }

                Type::Blank
            }
            ast::TypeExpr::Receiver(_) => Type::Receiver,
            ast::TypeExpr::Primitive(primitive_type) => Type::Primitive(primitive_type.value),
            ast::TypeExpr::Paren(type_expr) => self.type_expr(&type_expr.ty),
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

    fn local_ref(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.locals.get(name) {
                return Some(ty);
            }
        }
        None
    }

    fn local_mut(&mut self, name: &str) -> Option<&mut Type> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(ty) = scope.locals.get_mut(name) {
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
    scopes: SharedMut<Scopes>,
    source: Option<&'a str>,
}

impl<'a> TypeCheck<'a> {
    pub fn new() -> Self {
        Self {
            diagnostics: Default::default(),
            source: Default::default(),
            env: Env::new(),
            scopes: SharedMut::new(Default::default()),
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

    fn dump_scopes(&self) {
        for scope in self.scopes.scopes.iter() {
            for (name, ty) in scope.locals.iter() {
                println!("{}: {}", name, self.display_type(ty));
            }
        }
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
        left.collapse_nil();
        right.collapse_nil();

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
                            self.env.value_item_ref(ValueItemKind::Fn, *right)
                        else {
                            unreachable!()
                        };
                        self.cmp_bare_fns(left, &self.cast_fn_to_bare(right))
                    }
                    (Type::Fn(left), Type::Fn(right)) => {
                        let ValueItemRef::Fn(left) =
                            self.env.value_item_ref(ValueItemKind::Fn, *left)
                        else {
                            unreachable!()
                        };
                        let ValueItemRef::Fn(right) =
                            self.env.value_item_ref(ValueItemKind::Fn, *right)
                        else {
                            unreachable!()
                        };
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
                    (Type::Tuple(left), Type::Tuple(right)) => left
                        .types
                        .iter_mut()
                        .zip(right.types.iter_mut())
                        .all(|(l, r)| self.can_assing_type(l, r)),
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
                let ValueItemRef::Fn(func) = self.env.value_item_ref(ValueItemKind::Fn, *type_id)
                else {
                    unreachable!()
                };
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
            Type::Union(union) => union
                .types
                .iter()
                .map(|t| self.display_type(t))
                .join(" |")
                .to_string(),
            Type::Receiver => String::from("Self"),
            Type::Blank => String::from("Blank"),
            Type::Uninit(ty) => self.display_type(ty),
            Type::Tuple(tuple) => format!(
                "({})",
                tuple.types.iter().map(|t| self.display_type(t)).join(", ")
            ),
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

    fn unify_types(&self, left: Type, right: Type) -> Type {
        match (left, right) {
            (Type::Primitive(Primitive::Nil), right) => Type::Nilable(right.into()),
            (left, Type::Primitive(Primitive::Nil)) => Type::Nilable(left.into()),
            (left @ Type::Nilable(_), _) => left,
            (_, right @ Type::Nilable(_)) => right,
            (_, right) => right,
        }
    }

    fn expr(&mut self, expr: &ast::Expr, mut expected: Option<&mut Type>) -> Option<Type> {
        let mut ty = match &expr {
            ast::Expr::Lit(lit_expr) => match lit_expr {
                ast::LitExpr::Nil(_) => match expected.as_deref() {
                    Some(Type::Nilable(inner)) => Type::Nilable(inner.clone()),
                    _ => Type::Primitive(Primitive::Nil),
                },
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
                if let Some(ty) = self.scopes.local_ref(&ident.value) {
                    if !ty.is_initialised() {
                        self.add_error(
                            &format!("attempt to use uninitialised value `{}`", ident.value),
                            expr.span(),
                        );
                        return None;
                    }
                    ty.clone()
                } else if let Some((id, kind)) = self.env.value_items.get(&ident.value) {
                    match kind {
                        ValueItemKind::Fn => Type::Fn(*id),
                        ValueItemKind::Static => {
                            if let Some(ComplexType::Static(static_ty)) =
                                self.env.complex_types.get(id)
                            {
                                static_ty.ty.clone().unwrap()
                            } else {
                                self.add_error(
                                    &format!("cannot find value `{}` in this scope", &ident.value),
                                    expr.span(),
                                );
                                return None;
                            }
                        }
                    }
                } else {
                    self.add_error(
                        &format!("cannot find value `{}` in this scope", &ident.value),
                        expr.span(),
                    );
                    return None;
                }
            }
            ast::Expr::Tuple(tuple_expr) => {
                let exprs = tuple_expr.exprs.iter();
                let types = if let Some(Type::Tuple(tuple_expected)) = expected.as_deref_mut() {
                    exprs
                        .zip(tuple_expected.types.iter_mut())
                        .map(|(expr, expected)| self.expr(expr, Some(expected)))
                        .collect::<Option<Vec<_>>>()?
                } else {
                    exprs
                        .map(|e| self.expr(e, None))
                        .collect::<Option<Vec<_>>>()?
                };
                Type::Tuple(Tuple { types })
            }
            ast::Expr::Assign(assign_expr) => {
                match assign_expr.left.as_ref() {
                    ast::Expr::Ident(ident) => {
                        let mut scopes = self.scopes.clone();
                        let Some(local) = scopes.local_mut(&ident.value) else {
                            self.add_error(
                                &format!("cannot find value '{}' in this scope", &ident.value),
                                ident.span,
                            );
                            return None;
                        };
                        _ = self.expr(&assign_expr.right, Some(local))?;
                    }
                    _ => todo!(),
                }
                Type::Primitive(Primitive::Nil)
            }
            ast::Expr::Block(block_expr) => {
                let mut ty = Type::Primitive(Primitive::Nil);
                for (pos, stmt) in block_expr.stmts.iter().with_position() {
                    match pos {
                        Position::First | Position::Middle => {
                            self.stmt(stmt)?;
                        }
                        Position::Last | Position::Only => match stmt {
                            ast::Stmt::Expr(expr_stmt) => {
                                let expr_ty = self.expr(&expr_stmt.expr, None)?;
                                if expr_stmt.semi.is_none() {
                                    ty = expr_ty
                                }
                            }
                            _ => {
                                self.stmt(stmt)?;
                            }
                        },
                    }
                }
                ty
            }
            ast::Expr::If(if_expr) => {
                let _cond = self.expr(&if_expr.condition, None)?;
                let mut value = Type::Nilable(self.expr(&if_expr.value, None)?.into());
                value.collapse_nil();
                value
            }
            ast::Expr::Binary(binary_expr) => match binary_expr.op {
                BinaryOp::Else => {
                    if let ast::Expr::If(if_expr) = &*binary_expr.left {
                        let left = self.expr(&if_expr.value, None)?;
                        let right = self.expr(&binary_expr.right, None)?;
                        self.unify_types(left, right)
                    } else {
                        let mut left = self.expr(&binary_expr.left, None)?;
                        let mut right = self.expr(&binary_expr.right, None)?;
                        match &mut left {
                            Type::Primitive(Primitive::Nil) => right,
                            Type::Nilable(inner) => {
                                if !self.can_assing_type(inner, &mut right) {
                                    self.add_assign_error(&left, &right, binary_expr.left.span());
                                    return None;
                                }
                                right
                            }
                            other => {
                                if !self.can_assing_type(other, &mut right) {
                                    self.add_assign_error(&left, &right, binary_expr.left.span());
                                    return None;
                                }
                                left
                            }
                        }
                    }
                    // right
                }
                _ => todo!(),
            },
            ast::Expr::Call(call_expr) => todo!(),
            ast::Expr::Closure(closure_expr) => todo!(),
            ast::Expr::For(for_expr) => todo!(),
            ast::Expr::FieldGet(field_get_expr) => todo!(),
            ast::Expr::Group(group_expr) => todo!(),
            ast::Expr::Index(index_expr) => todo!(),
            ast::Expr::Loop(loop_expr) => todo!(),
            ast::Expr::MethodCall(method_call_expr) => todo!(),
            ast::Expr::Struct(struct_expr) => todo!(),
            ast::Expr::Path(path) => todo!(),
            ast::Expr::Unary(unary_expr) => todo!(),
            ast::Expr::While(while_expr) => todo!(),
            ast::Expr::Break(break_expr) => todo!(),
            ast::Expr::Continue(continue_expr) => todo!(),
            ast::Expr::Return(return_expr) => todo!(),
            ast::Expr::Yield(yield_expr) => todo!(),
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

    fn add_assign_error(&mut self, left: &Type, right: &Type, span: position::Span) {
        self.add_error(
            &format!(
                "could not assign {} to {}",
                self.display_type(right),
                self.display_type(left)
            ),
            span,
        );
    }

    fn type_expr(&mut self, type_expr: &ast::TypeExpr) -> Option<Type> {
        let span = type_expr.span();
        Some(match type_expr {
            ast::TypeExpr::Array(type_expr) => {
                let inner = self.type_expr(type_expr)?;
                Type::Array(inner.into())
            }
            ast::TypeExpr::BareFn(bare_fn) => Type::BareFn(BareFn {
                output: match &bare_fn.output {
                    ast::ReturnType::None => ReturnType::None,
                    ast::ReturnType::Type(type_exprs) => ReturnType::Type(
                        type_exprs
                            .iter()
                            .map(|ty| self.type_expr(ty))
                            .collect::<Option<Vec<_>>>()?,
                    ),
                },
                params: bare_fn
                    .params
                    .iter()
                    .map(|p| match p {
                        ast::BareFnParam::Receiver(_) => Some(BareFnParam {
                            name: None,
                            ty: Type::Receiver,
                        }),
                        ast::BareFnParam::Typed(param) => {
                            self.type_expr(&param.ty).map(|ty| BareFnParam {
                                name: param.ident.as_ref().map(|i| i.value.clone()),
                                ty,
                            })
                        }
                    })
                    .collect::<Option<Vec<_>>>()?,
                variadic: {
                    if let Some(variadic) = bare_fn.variadic.as_ref() {
                        let ty = self.type_expr(&variadic.ty)?;
                        Some(
                            Variadic {
                                name: variadic.ident.as_ref().map(|i| i.value.clone()),
                                ty,
                            }
                            .into(),
                        )
                    } else {
                        None
                    }
                },
            }),
            ast::TypeExpr::Nilable(type_expr) => Type::Nilable(self.type_expr(type_expr)?.into()),
            ast::TypeExpr::Path(path) => {
                //TODO: add proper modules support
                //what about
                let name = &path.segments[0].ident.value;

                if let Some((id, kind)) = self.env.type_items.get(name.as_str()) {
                    match kind {
                        TypeItemKind::Struct => Type::Struct(*id),
                        TypeItemKind::Enum => Type::Enum(*id),
                        TypeItemKind::Trait => todo!(),
                    }
                } else if let Some((id, kind)) = self.env.value_items.get(name.as_str()) {
                    match kind {
                        ValueItemKind::Fn => Type::Fn(*id),
                        ValueItemKind::Static => {
                            todo!()
                        }
                    }
                } else {
                    self.add_error(&format!("could not find type {name}"), span);
                    return None;
                }
            }
            ast::TypeExpr::Tuple(tuple_type) => Type::Tuple(Tuple {
                types: tuple_type
                    .types
                    .iter()
                    .map(|t| self.type_expr(t))
                    .collect::<Option<Vec<_>>>()?,
            }),
            ast::TypeExpr::Receiver(_) => Type::Receiver,
            ast::TypeExpr::Primitive(primitive_type) => Type::Primitive(primitive_type.value),
            ast::TypeExpr::Paren(type_expr) => self.type_expr(&type_expr.ty)?,
            ast::TypeExpr::Union(union_type) => {
                let mut types = vec![];
                let mut head = &*union_type.right;

                while let ast::TypeExpr::Union(ast::UnionType { left, right, .. }) = head {
                    types.push(self.type_expr(left)?);
                    head = right;
                }

                types.push(self.type_expr(head)?);
                Type::Union(Union { types })
            }
        })
    }

    fn binding_inner(
        &mut self,
        pat: &ast::Pat,
        expr: Option<&ast::Expr>,
        ty: Option<(Type, position::Span)>,
    ) -> Option<Type> {
        match (ty, expr) {
            (None, None) => {
                self.add_error(&format!("expected type or value for {:?}", pat), pat.span());
                return None;
            }
            (None, Some(expr)) => match pat {
                ast::Pat::Ident(ast::PatIdent { value: ident, .. }) => {
                    let ty = self.expr(expr, None)?;
                    self.scopes.insert_local(&ident.value, ty);
                }
                ast::Pat::Paren(pat_paren) => {
                    self.binding_inner(&pat_paren.pat, Some(expr), None)?;
                }
                ast::Pat::Tuple(pat_tuple) => {
                    let ast::Expr::Tuple(tuple_expr) = expr else {
                        self.add_error(
                            &format!("expected tuple, got {}", self.source_substr(expr.span())),
                            expr.span(),
                        );
                        return None;
                    };
                    if tuple_expr.exprs.len() != pat_tuple.pats.len() {
                        self.add_error(
                            &format!(
                                "expected tuple of length {}, got one of length {}",
                                pat_tuple.pats.len(),
                                tuple_expr.exprs.len()
                            ),
                            expr.span(),
                        );
                        return None;
                    }

                    for (expr, pat) in tuple_expr.exprs.iter().zip(pat_tuple.pats.iter()) {
                        self.binding_inner(pat, Some(expr), None)?;
                    }
                }
                ast::Pat::Path(path) => todo!(),
            },
            (Some((ty, ty_span)), None) => match pat {
                ast::Pat::Ident(ast::PatIdent { value: ident, .. }) => {
                    let ty = Type::Uninit(ty.into());
                    self.scopes.insert_local(&ident.value, ty);
                }
                ast::Pat::Tuple(pat_tuple) => {
                    let Type::Tuple(tuple_ty) = ty else {
                        self.add_error(
                            &format!("expected tuple, got {}", self.display_type(&ty)),
                            pat_tuple.span,
                        );
                        return None;
                    };
                    if tuple_ty.types.len() != pat_tuple.pats.len() {
                        self.add_error(
                            &format!(
                                "expected tuple of length {}, got one of length {}",
                                pat_tuple.pats.len(),
                                tuple_ty.types.len()
                            ),
                            ty_span,
                        );
                        return None;
                    }

                    for (ty, pat) in tuple_ty.types.iter().zip(pat_tuple.pats.iter()) {
                        self.binding_inner(pat, None, Some((ty.clone(), ty_span)));
                    }
                }
                ast::Pat::Paren(pat_paren) => {
                    self.binding_inner(&pat_paren.pat, None, None)?;
                }
                ast::Pat::Path(path) => todo!(),
            },
            (Some((mut expected, ty_span)), Some(expr)) => match pat {
                ast::Pat::Ident(ast::PatIdent { value: ident, .. }) => {
                    let expr_ty = self.expr(expr, Some(&mut expected))?;
                    self.scopes.insert_local(&ident.value, expr_ty);
                }
                ast::Pat::Tuple(pat_tuple) => {
                    let (Type::Tuple(tuple_ty), ast::Expr::Tuple(tuple_expr)) = (&expected, expr)
                    else {
                        self.add_error(
                            &format!("expected tuple, got {}", self.display_type(&expected)),
                            pat_tuple.span,
                        );
                        return None;
                    };
                    if tuple_ty.types.len() != pat_tuple.pats.len() {
                        self.add_error(
                            &format!(
                                "expected tuple of length {}, got one of length {}",
                                pat_tuple.pats.len(),
                                tuple_ty.types.len()
                            ),
                            ty_span,
                        );
                        return None;
                    }

                    if tuple_expr.exprs.len() != pat_tuple.pats.len() {
                        self.add_error(
                            &format!(
                                "expected tuple of length {}, got one of length {}",
                                pat_tuple.pats.len(),
                                tuple_expr.exprs.len()
                            ),
                            expr.span(),
                        );
                        return None;
                    }

                    for ((expected, expr), pat) in tuple_ty
                        .types
                        .iter()
                        .zip(tuple_expr.exprs.iter())
                        .zip(pat_tuple.pats.iter())
                    {
                        self.binding_inner(pat, Some(expr), Some((expected.clone(), ty_span)))?;
                    }
                }
                ast::Pat::Paren(pat_paren) => {
                    self.binding_inner(&pat_paren.pat, Some(expr), Some((expected, ty_span)));
                }
                ast::Pat::Path(path) => todo!(),
            },
        };
        Some(Type::Primitive(Primitive::Nil))
    }

    fn binding(&mut self, binding: &ast::BindingStmt) -> Option<Type> {
        let ty = match binding.ty.as_ref().map(|t| {
            let span = t.span();
            self.type_expr(t).map(|t| (t, span))
        }) {
            Some(Some(t)) => Some(t),
            Some(None) => return None,
            None => None,
        };
        self.binding_inner(&binding.pat, binding.expr.as_ref(), ty)
    }

    fn stmt(&mut self, stmt: &ast::Stmt) -> Option<Type> {
        Some(match stmt {
            ast::Stmt::Expr(expr) => {
                self.expr(&expr.expr, None)?;
                Type::Primitive(Primitive::Nil)
            }
            ast::Stmt::Binding(binding) => self.binding(binding)?,
        })
        // Some(match stmt {
        //     ast::Stmt::Expr(expr_stmt) => match expr_stmt.exprs.as_slice() {
        //         [expr] => self.expr(expr, None)?,
        //         exprs => Type::Block(
        //             exprs
        //                 .iter()
        //                 .map(|e| self.expr(e, None))
        //                 .collect::<Option<Vec<_>>>()?,
        //         ),
        //     },
        //     ast::Stmt::Assign(assign_stmt) => todo!(),
        //     ast::Stmt::BinaryAssign(binary_assign_stmt) => todo!(),
        //     ast::Stmt::Binding(binding) => self.binding(binding)?,
        //     ast::Stmt::Return(return_stmt) => todo!(),
        //     ast::Stmt::Continue(continue_stmt) => todo!(),
        //     ast::Stmt::Break(break_stmt) => todo!(),
        //     ast::Stmt::Yield(yield_stmt) => todo!(),
        //     ast::Stmt::Empty(empty_stmt) => todo!(),
        // })
    }

    fn fn_item(&mut self, item_fn: &ast::ItemFn) -> Option<()> {
        let fn_id = self.env.types_by_ids.get(&item_fn.id).unwrap();
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

        for stmt in item_fn.body.stmts.iter() {
            self.stmt(stmt)?;
        }

        self.dump_scopes();

        self.scopes.pop_scope();
        self.context.current_fn = None;

        Some(())
    }

    fn static_item(&mut self, static_item: &ast::ItemStatic) -> Option<()> {
        let Some((id, _)) = self.env.value_items.get(&static_item.ident.value) else {
            unreachable!()
        };
        let mut complex_types = self.env.complex_types.clone();
        let Some(ComplexType::Static(static_ty)) = complex_types.get_mut(id) else {
            unreachable!()
        };
        if let Some(ty) = &mut static_ty.ty {
            self.expr(&static_item.expr, Some(ty))?;
        } else {
            let expr_ty = self.expr(&static_item.expr, None)?;
            static_ty.ty = Some(expr_ty);
        }
        Some(())
    }

    fn impl_item(&mut self, item_impl: &ast::ItemImpl) -> Option<()> {
        Some(())
    }

    fn item(&mut self, item: &ast::Item) -> Option<()> {
        match item {
            ast::Item::Fn(item_fn) => self.fn_item(item_fn),
            ast::Item::Impl(item_impl) => self.impl_item(item_impl),
            ast::Item::Static(item_static) => self.static_item(item_static),
            _ => Some(()),
        }
    }

    fn push_global_scope(&mut self) {
        let complex_types = self.env.complex_types.clone();
        self.scopes.push_scope();
        for (name, (id, kind)) in &self.env.value_items {
            self.scopes.insert_local(
                name.as_str(),
                match kind {
                    ValueItemKind::Fn => Type::Fn(*id),
                    ValueItemKind::Static => {
                        let Some(ComplexType::Static(static_ty)) = complex_types.get(id) else {
                            unreachable!()
                        };
                        static_ty.ty.clone().unwrap()
                    }
                },
            );
        }
    }

    pub fn check(&mut self, program: &[ast::Item]) -> Option<()> {
        //TODO: check default values after collecting definitions
        self.env.collect(program)?;
        println!("collecting finished");

        //TODO: resolve statics as a separate step
        // self.push_global_scope();
        for item in program {
            self.item(item)?;
        }

        self.diagnostics.append(&mut self.env.diagnostics);
        Some(())
    }

    pub fn debug_dump(&self) {
        println!("value items: ");
        for (name, (id, kind)) in self.env.value_items.iter() {
            let ty = self.env.complex_types.get(id).unwrap();
            match kind {
                ValueItemKind::Fn => {
                    println!("{};", self.display_type(&Type::Fn(*id)));
                }
                ValueItemKind::Static => {
                    let ComplexType::Static(static_ty) = ty else {
                        unreachable!()
                    };
                    let ty = static_ty.ty.as_ref().unwrap();
                    println!("static {}: {}", name, self.display_type(ty));
                }
            }
        }
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

#[cfg(test)]
mod test {
    use crate::type_check::Type;

    #[test]
    fn collapse_nil() {
        let mut nested = Type::Nilable(Type::Nilable(Type::Receiver.into()).into());
        nested.collapse_nil();
        assert_eq!(nested, Type::Nilable(Type::Receiver.into()));
    }
}
