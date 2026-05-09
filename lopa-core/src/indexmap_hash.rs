#[macro_export]
macro_rules! indexmap_hash {
    (
        $(
            $name:ident$(<$generics:tt>)?($indexmap:ty)
        ),+ $(,)?
    ) => {
        $(
            #[derive(Clone, Default, salsa::Update, PartialEq, Eq)]
            #[repr(transparent)]
            pub struct $name $(<$generics>)?(pub $indexmap);
            impl$(<$generics>)? std::hash::Hash for $name $(<$generics>)? {
                fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                    self.0.iter().for_each(|e| e.hash(state));
                }
            }

            impl<'db> std::ops::Deref for $name<'db> {
                type Target = $indexmap;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl<'db> std::ops::DerefMut for $name<'db> {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
                }
            }
            // unsafe impl<'db> salsa::Update for $name<'db> {
            //     unsafe fn maybe_update(old_pointer: *mut Self, new_value: Self) -> bool {
            //         unsafe {
            //             <$indexmap>::maybe_update(
            //                 old_pointer as *mut _,
            //                 new_value.0,
            //             )
            //         }
            //     }
            // }
            //
        )+
    };
}
