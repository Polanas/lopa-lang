use la_arena::{Arena, ArenaMap};

use crate::{
    def::{
        AstId, ErasedAstId,
        hir::{Class, Enum, Function, ImplBlock, Module, ModuleData, Struct, Trait, UseItem},
    },
    ide::InFile,
    parsing,
};

#[derive(Debug, Clone, Default, PartialEq, salsa::SalsaValue)]
pub struct ItemsMap<'db> {
    fn_items: ArenaMap<ErasedAstId, Function<'db>>,
    struct_items: ArenaMap<ErasedAstId, Struct<'db>>,
    class_items: ArenaMap<ErasedAstId, Class<'db>>,
    trait_items: ArenaMap<ErasedAstId, Trait<'db>>,
    module_items: ArenaMap<ErasedAstId, Module<'db>>,
    impl_items: ArenaMap<ErasedAstId, ImplBlock<'db>>,
    enum_items: ArenaMap<ErasedAstId, Enum<'db>>,
    use_items: ArenaMap<ErasedAstId, UseItem<'db>>,
}

impl<'db> std::ops::Index<InFile<AstId<parsing::FnItem<'static>>>> for ItemsMap<'db> {
    type Output = Function<'db>;

    fn index(&self, index: InFile<AstId<parsing::FnItem<'static>>>) -> &Self::Output {
        self.fn_items.get(index.value.erased()).unwrap()
    }
}

impl<'db> std::ops::Index<InFile<AstId<parsing::StructItem<'static>>>> for ItemsMap<'db> {
    type Output = Struct<'db>;

    fn index(&self, index: InFile<AstId<parsing::StructItem<'static>>>) -> &Self::Output {
        self.struct_items.get(index.value.erased()).unwrap()
    }
}

impl<'db> std::ops::Index<InFile<AstId<parsing::ClassItem<'static>>>> for ItemsMap<'db> {
    type Output = Class<'db>;

    fn index(&self, index: InFile<AstId<parsing::ClassItem<'static>>>) -> &Self::Output {
        self.class_items.get(index.value.erased()).unwrap()
    }
}

impl<'db> std::ops::Index<InFile<AstId<parsing::TraitItem<'static>>>> for ItemsMap<'db> {
    type Output = Trait<'db>;

    fn index(&self, index: InFile<AstId<parsing::TraitItem<'static>>>) -> &Self::Output {
        self.trait_items.get(index.value.erased()).unwrap()
    }
}

impl<'db> std::ops::Index<InFile<AstId<parsing::ModItem<'static>>>> for ItemsMap<'db> {
    type Output = Module<'db>;

    fn index(&self, index: InFile<AstId<parsing::ModItem<'static>>>) -> &Self::Output {
        self.module_items.get(index.value.erased()).unwrap()
    }
}

impl<'db> std::ops::Index<InFile<AstId<parsing::ImplItem<'static>>>> for ItemsMap<'db> {
    type Output = ImplBlock<'db>;

    fn index(&self, index: InFile<AstId<parsing::ImplItem<'static>>>) -> &Self::Output {
        self.impl_items.get(index.value.erased()).unwrap()
    }
}

impl<'db> std::ops::Index<InFile<AstId<parsing::EnumItem<'static>>>> for ItemsMap<'db> {
    type Output = Enum<'db>;

    fn index(&self, index: InFile<AstId<parsing::EnumItem<'static>>>) -> &Self::Output {
        self.enum_items.get(index.value.erased()).unwrap()
    }
}

impl<'db> std::ops::Index<InFile<AstId<parsing::UseItem<'static>>>> for ItemsMap<'db> {
    type Output = UseItem<'db>;

    fn index(&self, index: InFile<AstId<parsing::UseItem<'static>>>) -> &Self::Output {
        self.use_items.get(index.value.erased()).unwrap()
    }
}

impl<'db> ItemsMap<'db> {
    pub fn insert_fn(&mut self, db: &'db dyn salsa::Database, item: Function<'db>) {
        self.fn_items.insert(item.id(db).value.erased(), item);
    }

    pub fn insert_struct(&mut self, db: &'db dyn salsa::Database, item: Struct<'db>) {
        self.struct_items.insert(item.id(db).value.erased(), item);
    }

    pub fn insert_module(&mut self, db: &'db dyn salsa::Database, item: Module<'db>) {
        assert!(!matches!(item.data(db), ModuleData::Root { .. }));

        if let Some(id) = item.id(db) {
            self.module_items.insert(id.value.erased(), item);
        }
    }

    pub fn insert_impl(&mut self, db: &'db dyn salsa::Database, item: ImplBlock<'db>) {
        self.impl_items.insert(item.id(db).value.erased(), item);
    }

    pub fn insert_enum(&mut self, db: &'db dyn salsa::Database, item: Enum<'db>) {
        self.enum_items.insert(item.id(db).value.erased(), item);
    }

    pub fn insert_use(&mut self, db: &'db dyn salsa::Database, item: UseItem<'db>) {
        self.use_items.insert(item.id(db).value.erased(), item);
    }

    pub fn insert_class(&mut self, db: &'db dyn salsa::Database, item: Class<'db>) {
        self.class_items.insert(item.id(db).value.erased(), item);
    }

    pub fn insert_trait(&mut self, db: &'db dyn salsa::Database, item: Trait<'db>) {
        self.trait_items.insert(item.id(db).value.erased(), item);
    }
}
