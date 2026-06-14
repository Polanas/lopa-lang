use std::path::{Path, PathBuf};

use itertools::Itertools;
use rowan::ast::AstNode;
use salsa::{Accumulator, Database};

use crate::{
    def::{
        ir::{self, BareFn, Function, Type},
        resolver, scope,
    },
    ide::{
        self,
        diagnostics::{self, Diagnostic, DiagnosticKind},
    },
    parsing::ast,
};

#[salsa::tracked(debug)]
pub struct ModuleItemData<'db> {
    #[returns(ref)]
    pub functions: Vec<ir::Function<'db>>,
    #[returns(ref)]
    pub structs: Vec<ir::Struct<'db>>,
    #[returns(ref)]
    pub enums: Vec<ir::Enum<'db>>,
    #[returns(ref)]
    pub use_imports: Vec<ir::UseItem<'db>>,
    #[returns(ref)]
    pub children: Vec<ir::Module<'db>>,
}

struct LowerCtx<'db> {
    db: &'db dyn salsa::Database,
    functions: Vec<ir::Function<'db>>,
    structs: Vec<ir::Struct<'db>>,
    enums: Vec<ir::Enum<'db>>,
    use_items: Vec<ir::UseItem<'db>>,
    impl_blocks: Vec<ir::ImplBlock<'db>>,
    children: Vec<ir::Module<'db>>,
    file: ide::File,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModuleErrorKind {
    UnresolvedModule,
    UndersolvedImport,
}

impl<'db> LowerCtx<'db> {
    pub fn new(db: &'db dyn salsa::Database, file: ide::File) -> Self {
        Self {
            functions: Default::default(),
            structs: Default::default(),
            impl_blocks: Default::default(),
            use_items: Default::default(),
            children: Default::default(),
            enums: Default::default(),
            file,
            db,
        }
    }

    pub fn lower_impls(mut self, file: ast::File) -> Vec<ir::ImplBlock<'db>> {
        for item in file.items().filter_map(|i| match i {
            ast::Item::ImplItem(i) => Some(i),
            _ => None,
        }) {
            if let Some(item) = self.impl_item(item) {
                self.impl_blocks.push(item);
            }
        }
        self.impl_blocks
    }

    pub fn lower_items(mut self, file: ast::File) -> ModuleItemData<'db> {
        for item in file.items() {
            self.item(item);
        }

        ModuleItemData::new(
            self.db,
            self.functions,
            self.structs,
            self.enums,
            self.use_items,
            self.children,
        )
    }

    fn impl_item(&mut self, item: ast::ImplItem) -> Option<ir::ImplBlock<'db>> {
        // let ty = item.ty().map(|i| lower_type_expr(self.db, self.file, i))?;
        // let impl_ty = item
        //     .impl_ty()
        //     .and_then(|t| t.ty())
        //     .map(|i| lower_type_expr(self.db, self.file, i));
        // let owner = impl_ty.as_ref().unwrap_or_else(|| &ty).clone();
        // let methods = item
        //     .functions()
        //     .filter_map(|f| self.fn_item(f, Some(owner.clone()), impl_ty.clone(), Some(item)))
        //     .collect_vec();
        Some(ir::ImplBlock::new(
            self.db,
            self.file,
            ast::AstPtr::new(&item),
        ))
    }

    fn item(&mut self, item: ast::Item) {
        match item {
            ast::Item::FnItem(fn_item) => {
                if let Some(item) = self.fn_item(fn_item, None) {
                    self.functions.push(item);
                }
            }
            ast::Item::StructItem(struct_item) => {
                self.struct_item(struct_item);
            }
            ast::Item::EnumItem(enum_item) => {
                self.enum_item(enum_item);
            }
            ast::Item::UseItem(use_item) => {
                self.use_items.push(self.use_item(use_item));
            }
            ast::Item::ModItem(mod_item) if mod_item.semi().is_some() => {
                if let Some(module) = self.resolve_module(mod_item.clone(), self.file) {
                    if module == self.file {
                        Diagnostic::new(
                            mod_item.syntax().text_range(),
                            diagnostics::DiagnosticKind::ModuleError,
                            format!(
                                "cyclic definition: `{}`",
                                mod_item
                                    .name()
                                    .and_then(|n| n.text())
                                    .unwrap_or_else(|| "?".into())
                            ),
                        )
                        .accumulate(self.db);
                    } else {
                        self.children.push(ir::Module::new(
                            self.db,
                            module,
                            ast::AstPtr::new(&mod_item),
                        ));
                    }
                } else {
                    Diagnostic::new(
                        mod_item.syntax().text_range(),
                        diagnostics::DiagnosticKind::ModuleError,
                        format!(
                            "unresolved module `{}`",
                            mod_item
                                .name()
                                .and_then(|n| n.text())
                                .unwrap_or_else(|| "?".into())
                        ),
                    )
                    .accumulate(self.db);
                }
            }
            _ => {}
        };
    }

    fn resolve_module(&self, mod_item: ast::ModItem, parent: ide::File) -> Option<ide::File> {
        let root = parent.source_root(self.db);
        let files = root.files(self.db)?;
        let name = mod_item.name().and_then(|n| n.text())?;

        let mod_path = if ide::is_root_file(self.db, parent) {
            Path::new(parent.path(self.db).0.as_path())
                .parent()?
                .join(format!("{name}.lopa"))
        } else {
            Path::new(parent.path(self.db).0.as_path())
                .parent()?
                .join(ide::module_name(self.db, parent))
                .join(format!("{name}.lopa"))
        };
        files
            .iter()
            .find(|f| f.path(self.db).0.as_path() == mod_path.as_path())
            .cloned()
    }

    fn use_item(&self, use_item: ast::UseItem) -> ir::UseItem<'db> {
        ir::UseItem::new(self.db, ast::AstPtr::new(&use_item))
    }

    fn enum_item(&mut self, enum_item: ast::EnumItem) -> Option<()> {
        for elem in enum_item.elements() {
            if let ast::EnumElem::Field(field) = elem
                && let Some(ty) = field.ty()
            {
                match ty {
                    ast::ItemTypeExpr::StructItemType(struct_item_type)
                        if let Some(struct_item) = struct_item_type.struct_item() =>
                    {
                        self.struct_item(struct_item);
                    }
                    ast::ItemTypeExpr::EnumItemType(enum_item_type)
                        if let Some(enum_item) = enum_item_type.enum_item() =>
                    {
                        self.enum_item(enum_item);
                    }
                    _ => {}
                }
            }
        }
        self.enums.push(ir::Enum::new(
            self.db,
            enum_item.name()?.text()?,
            ast::AstPtr::new(&enum_item),
            self.file,
        ));
        Some(())
    }

    fn struct_item(&mut self, struct_item: ast::StructItem) -> Option<()> {
        for elem in struct_item.elements() {
            if let ast::StructElem::Field(field) = elem
                && let Some(ty) = field.ty()
            {
                match ty {
                    ast::ItemTypeExpr::StructItemType(struct_item_type)
                        if let Some(struct_item) = struct_item_type.struct_item() =>
                    {
                        self.struct_item(struct_item);
                    }
                    ast::ItemTypeExpr::EnumItemType(enum_item_type)
                        if let Some(enum_item) = enum_item_type.enum_item() =>
                    {
                        self.enum_item(enum_item);
                    }
                    _ => {}
                };
            }
        }
        self.structs.push(ir::Struct::new(
            self.db,
            struct_item.name()?.text()?,
            ast::AstPtr::new(&struct_item),
            self.file,
        ));
        Some(())
    }

    fn fn_item(
        &self,
        fn_item: ast::FnItem,
        owner: Option<ir::FunctionOwnerItem<'db>>,
    ) -> Option<ir::Function<'db>> {
        Some(ir::Function::new(
            self.db,
            fn_item.name()?.text()?,
            ast::AstPtr::new(&fn_item),
            self.file,
            owner,
            None,
        ))
    }
}


#[salsa::tracked]
pub fn module_parent(db: &dyn salsa::Database, file: ide::File) -> Option<ide::File> {
    module_parents(db, file.source_root(db))
        .get(&file)
        .cloned()?
}

#[salsa::tracked]
fn module_parents(
    db: &dyn salsa::Database,
    source_root: ide::SourceRoot,
) -> indexmap::IndexMap<ide::File, Option<ide::File>> {
    fn module_parents_inner(
        db: &dyn salsa::Database,
        parent: ide::File,
        parents: &mut indexmap::IndexMap<ide::File, Option<ide::File>>,
    ) {
        for child in module_items(db, parent).children(db) {
            //TODO: store Modules instead of Files
            module_parents_inner(db, child.file(db), parents);
            parents.insert(child.file(db), Some(parent));
        }
    }
    let mut parents: indexmap::IndexMap<ide::File, Option<ide::File>> = Default::default();
    let root = ide::root_module(db, source_root).unwrap();
    parents.insert(root, None);
    module_parents_inner(db, root, &mut parents);
    parents
}

#[salsa::tracked]
pub fn module_items<'db>(db: &'db dyn salsa::Database, file: ide::File) -> ModuleItemData<'db> {
    let parse = ide::parse(db, file);
    let ctx = LowerCtx::new(db, file);
    ctx.lower_items(parse.file(db))
}

#[salsa::tracked]
pub fn impl_blocks<'db>(db: &'db dyn salsa::Database, file: ide::File) -> Vec<ir::ImplBlock<'db>> {
    let parse = ide::parse(db, file);
    let ctx = LowerCtx::new(db, file);
    ctx.lower_impls(parse.file(db))
}
// #[salsa::tracked(returns(ref))]
// pub fn lower_module<'db>(db: &'db dyn salsa::Database, file: ide::File) -> MoudleIr<'db> {
//     MoudleIr::new(db, lower_structs_fns(db, file), lower_impl_blocks(db, file))
// }
