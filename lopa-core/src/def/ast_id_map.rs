use std::{fmt::Debug, marker::PhantomData, process::Output};

use itertools::Itertools;
use la_arena::{Arena, Idx};

use crate::parsing::{self, AstNode};

pub type ErasedAstId = Idx<parsing::NodeId>;

//Primary purpose of AstId is to be used for diagnostics.
//The ids themselves are stored inside items.
//They can then be sent via accumulators to then be converted into NodeId's
#[derive(salsa::Update, Clone, Copy)]
pub struct AstId<A: parsing::AstNode<'static>>(ErasedAstId, PhantomData<A>);

impl<A: parsing::AstNode<'static>> AstId<A> {
    pub fn erased(&self) -> ErasedAstId {
        self.0
    }
}

impl<A: parsing::AstNode<'static>> From<ErasedAstId> for AstId<A> {
    fn from(value: ErasedAstId) -> Self {
        Self(value, PhantomData)
    }
}

impl<A: parsing::AstNode<'static>> Eq for AstId<A> {}

impl<A: parsing::AstNode<'static>> std::hash::Hash for AstId<A> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<A: parsing::AstNode<'static>> PartialEq for AstId<A> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl<A: parsing::AstNode<'static>> Debug for AstId<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AstPtr").field(&self.0).finish()
    }
}

#[derive(salsa::Update, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum AstIdKind {
    Function,
    Mod,
    Impl,
    Struct,
    Enum,
    Use,
}

pub trait AstIdItem<'a>: parsing::AstNode<'a> + Sized {
    type Output: Sized + 'static;
    const KIND: AstIdKind;
    fn as_ast_id(id: ErasedAstId) -> Self::Output;
}

macro_rules! impl_ast_id_item {
    (
        $([$ty:ident, $kind:ident]),* $(,)?
    ) => {
        $(
            impl<'a> AstIdItem<'a> for parsing::$ty<'a> {
                type Output = AstId<parsing::$ty<'static>>;
                const KIND: AstIdKind = AstIdKind::$kind;
                fn as_ast_id(id: ErasedAstId) -> Self::Output {
                    AstId(id, PhantomData)
                }
            }

            impl std::ops::Index<AstId<parsing::$ty<'static>>> for AstIdMap {
                type Output = parsing::NodeId;

                fn index(&self, index: AstId<parsing::$ty<'static>>) -> &Self::Output {
                    let arena = &self.arenas[AstIdKind::$kind as u8 as usize];
                    &arena[index.0]
                }
            }
        )*
    };
}

impl_ast_id_item! {
    [FnItem, Function],
    [ModItem, Mod],
    [ImplItem, Impl],
    [StructItem, Struct],
    [EnumItem, Enum],
    [UseItem, Use]
}

#[derive(salsa::Update, PartialEq, Clone)]
pub struct AstIdMap {
    arenas: Vec<Arena<parsing::NodeId>>,
}

impl AstIdMap {
    pub fn new() -> Self {
        Self {
            arenas: (0..=(AstIdKind::Use as usize))
                .map(|_| Arena::new())
                .collect_vec(),
        }
    }

    pub fn insert<'a, A: AstIdItem<'a>>(&mut self, item: A) -> A::Output {
        let arena = &mut self.arenas[A::KIND as u8 as usize];
        A::as_ast_id(arena.alloc(item.id()))
    }

    pub fn get<A: AstIdItem<'static>>(&self, id: AstId<A>) -> Option<parsing::NodeId> {
        let arena = &self.arenas[A::KIND as u8 as usize];
        arena.get(id.0).copied()
    }
}

impl Default for AstIdMap {
    fn default() -> Self {
        Self::new()
    }
}
