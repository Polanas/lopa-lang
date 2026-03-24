use std::{
    cell::UnsafeCell,
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
    rc::{Rc, Weak},
};

pub struct SharedMut<T>(Rc<UnsafeCell<T>>);

impl<T: 'static> From<T> for SharedMut<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: 'static> Clone for SharedMut<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: 'static + Display> Display for SharedMut<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (unsafe { &*self.0.get() }).fmt(f)
    }
}

impl<T: 'static + Debug> Debug for SharedMut<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (unsafe { &*self.0.get() }).fmt(f)
    }
}

impl<T: 'static> Deref for SharedMut<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.get() }
    }
}

impl<T: 'static> DerefMut for SharedMut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.get() }
    }
}

impl<T> PartialEq for SharedMut<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: 'static> SharedMut<T> {
    pub fn new(value: T) -> Self {
        Self(Rc::new(UnsafeCell::new(value)))
    }

    pub fn inner(shared_mut: &SharedMut<T>) -> &Rc<UnsafeCell<T>> {
        &shared_mut.0
    }

    pub fn downgrade(&self) -> WeakMut<T> {
        WeakMut(Rc::downgrade(&self.0))
    }

    pub fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    pub fn weak_count(&self) -> usize {
        Rc::weak_count(&self.0)
    }
}

pub struct WeakMut<T: 'static>(Weak<UnsafeCell<T>>);

impl<T: 'static> WeakMut<T> {
    pub fn upgrade(&self) -> Option<SharedMut<T>> {
        self.0.upgrade().map(SharedMut)
    }
    pub fn strong_count(&self) -> usize {
        Weak::strong_count(&self.0)
    }

    pub fn weak_count(&self) -> usize {
        Weak::weak_count(&self.0)
    }
}

impl<T: 'static> Clone for WeakMut<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> PartialEq for WeakMut<T> {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.0, &other.0)
    }
}
