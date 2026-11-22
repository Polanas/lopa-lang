use la_arena::ArenaMap;

use crate::def::{
    ErasedAstId, Symbol,
    hir::{
        Class, ClassId, Enum, EnumId, Function, FunctionId, ImplBlock, ImplBlockId, Module,
        ModuleId, Struct, StructId, Trait, TraitId, UseItem, UseItemId,
    },
};

#[derive(salsa::SalsaValue, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ItemContainer<'db> {
    Module(Module<'db>),
    Class(Class<'db>),
    Enum(Enum<'db>),
    Struct(Struct<'db>),
    Trait(Trait<'db>),
    Impl(ImplBlock<'db>),
}

impl<'db> ItemContainer<'db> {
    pub fn name(&self, db: &'db dyn salsa::Database) -> Option<Symbol> {
        Some(match self {
            ItemContainer::Module(item) => item.name(db),
            ItemContainer::Class(item) => item.name(db),
            ItemContainer::Enum(item) => item.name(db),
            ItemContainer::Struct(item) => item.name(db),
            ItemContainer::Trait(item) => item.name(db),
            ItemContainer::Impl(_) => return None,
        })
    }
}

#[derive(salsa::SalsaValue, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ItemData<'db, I> {
    pub item: I,
    pub container: ItemContainer<'db>,
}

#[derive(Debug, Clone, Default, PartialEq, salsa::SalsaValue)]
pub struct ItemsMap<'db> {
    module_items: ArenaMap<ErasedAstId, ItemData<'db, Module<'db>>>,
    struct_items: ArenaMap<ErasedAstId, ItemData<'db, Struct<'db>>>,
    class_items: ArenaMap<ErasedAstId, ItemData<'db, Class<'db>>>,
    trait_items: ArenaMap<ErasedAstId, ItemData<'db, Trait<'db>>>,
    impl_items: ArenaMap<ErasedAstId, ItemData<'db, ImplBlock<'db>>>,
    enum_items: ArenaMap<ErasedAstId, ItemData<'db, Enum<'db>>>,
    use_items: ArenaMap<ErasedAstId, ItemData<'db, UseItem<'db>>>,
    fn_items: ArenaMap<ErasedAstId, ItemData<'db, Function<'db>>>,
}

impl<'db> std::ops::Index<FunctionId> for ItemsMap<'db> {
    type Output = ItemData<'db, Function<'db>>;

    fn index(&self, index: FunctionId) -> &Self::Output {
        self.fn_items.get(index.value.erased()).unwrap()
    }
}

impl<'db> std::ops::Index<StructId> for ItemsMap<'db> {
    type Output = ItemData<'db, Struct<'db>>;

    fn index(&self, index: StructId) -> &Self::Output {
        self.struct_items.get(index.value.erased()).unwrap()
    }
}

impl<'db> std::ops::Index<ClassId> for ItemsMap<'db> {
    type Output = ItemData<'db, Class<'db>>;

    fn index(&self, index: ClassId) -> &Self::Output {
        self.class_items.get(index.value.erased()).unwrap()
    }
}

impl<'db> std::ops::Index<TraitId> for ItemsMap<'db> {
    type Output = ItemData<'db, Trait<'db>>;

    fn index(&self, index: TraitId) -> &Self::Output {
        self.trait_items.get(index.value.erased()).unwrap()
    }
}

impl<'db> std::ops::Index<ModuleId> for ItemsMap<'db> {
    type Output = ItemData<'db, Module<'db>>;

    fn index(&self, index: ModuleId) -> &Self::Output {
        self.module_items.get(index.value.erased()).unwrap()
    }
}

impl<'db> std::ops::Index<ImplBlockId> for ItemsMap<'db> {
    type Output = ItemData<'db, ImplBlock<'db>>;

    fn index(&self, index: ImplBlockId) -> &Self::Output {
        self.impl_items.get(index.value.erased()).unwrap()
    }
}

impl<'db> std::ops::Index<EnumId> for ItemsMap<'db> {
    type Output = ItemData<'db, Enum<'db>>;

    fn index(&self, index: EnumId) -> &Self::Output {
        self.enum_items.get(index.value.erased()).unwrap()
    }
}

impl<'db> std::ops::Index<UseItemId> for ItemsMap<'db> {
    type Output = ItemData<'db, UseItem<'db>>;

    fn index(&self, index: UseItemId) -> &Self::Output {
        self.use_items.get(index.value.erased()).unwrap()
    }
}

impl<'db> ItemsMap<'db> {
    pub fn insert_fn(&mut self, db: &'db dyn salsa::Database, data: ItemData<'db, Function<'db>>) {
        self.fn_items.insert(data.item.id(db).value.erased(), data);
    }

    pub fn insert_struct(
        &mut self,
        db: &'db dyn salsa::Database,
        data: ItemData<'db, Struct<'db>>,
    ) {
        self.struct_items
            .insert(data.item.id(db).value.erased(), data);
    }

    pub fn insert_module(
        &mut self,
        db: &'db dyn salsa::Database,
        data: ItemData<'db, Module<'db>>,
    ) {
        if let Some(id) = data.item.id(db) {
            self.module_items.insert(id.value.erased(), data);
        }
    }

    pub fn insert_impl(
        &mut self,
        db: &'db dyn salsa::Database,
        data: ItemData<'db, ImplBlock<'db>>,
    ) {
        self.impl_items
            .insert(data.item.id(db).value.erased(), data);
    }

    pub fn insert_enum(&mut self, db: &'db dyn salsa::Database, data: ItemData<'db, Enum<'db>>) {
        self.enum_items
            .insert(data.item.id(db).value.erased(), data);
    }

    pub fn insert_use(&mut self, db: &'db dyn salsa::Database, data: ItemData<'db, UseItem<'db>>) {
        self.use_items.insert(data.item.id(db).value.erased(), data);
    }

    pub fn insert_class(&mut self, db: &'db dyn salsa::Database, data: ItemData<'db, Class<'db>>) {
        self.class_items
            .insert(data.item.id(db).value.erased(), data);
    }

    pub fn insert_trait(&mut self, db: &'db dyn salsa::Database, data: ItemData<'db, Trait<'db>>) {
        self.trait_items
            .insert(data.item.id(db).value.erased(), data);
    }
}
