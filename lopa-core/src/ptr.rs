use std::{
    cell::UnsafeCell,
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
    rc::{Rc, Weak},
};

pub struct Ptr<T>(Rc<UnsafeCell<T>>);

impl<T> Ptr<T> {
    pub fn try_unwrap(self) -> Option<T> {
        Rc::try_unwrap(self.0).ok().map(|t| t.into_inner())
    }
}

impl<T: 'static> From<T> for Ptr<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: 'static> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: 'static + Display> Display for Ptr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (unsafe { &*self.0.get() }).fmt(f)
    }
}

impl<T: 'static + Debug> Debug for Ptr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (unsafe { &*self.0.get() }).fmt(f)
    }
}

impl<T: 'static> Deref for Ptr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.get() }
    }
}

impl<T: 'static> DerefMut for Ptr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.get() }
    }
}

impl<T> PartialEq for Ptr<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: 'static> Ptr<T> {
    pub fn new(value: T) -> Self {
        Self(Rc::new(UnsafeCell::new(value)))
    }

    pub fn inner(shared_mut: &Ptr<T>) -> &Rc<UnsafeCell<T>> {
        &shared_mut.0
    }

    pub fn downgrade(&self) -> WeakPtr<T> {
        WeakPtr(Rc::downgrade(&self.0))
    }

    pub fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    pub fn weak_count(&self) -> usize {
        Rc::weak_count(&self.0)
    }
}

pub struct WeakPtr<T: 'static>(Weak<UnsafeCell<T>>);

impl<T: 'static> WeakPtr<T> {
    pub fn upgrade(&self) -> Option<Ptr<T>> {
        self.0.upgrade().map(Ptr)
    }
    pub fn strong_count(&self) -> usize {
        Weak::strong_count(&self.0)
    }

    pub fn weak_count(&self) -> usize {
        Weak::weak_count(&self.0)
    }
}

impl<T: 'static> Clone for WeakPtr<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> PartialEq for WeakPtr<T> {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.0, &other.0)
    }
}
