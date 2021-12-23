use std::{
    borrow::{Borrow, BorrowMut},
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct ChangedFlag(pub bool);

// Changed flag
#[derive(Debug)]
pub struct Changed<T> {
    pub data: T,
    flag: AtomicBool,
}

impl<T> Clone for Changed<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Changed {
            data: self.data.clone(),
            flag: AtomicBool::new(self.flag.load(Ordering::Relaxed)),
        }
    }
}

impl<T> Changed<T> {
    pub fn new(data: T, changed: bool) -> Self {
        Changed {
            data,
            flag: AtomicBool::new(changed),
        }
    }

    pub fn into_inner(self) -> T {
        self.data
    }
}

impl<T> Borrow<T> for Changed<T> {
    fn borrow(&self) -> &T {
        &self.data
    }
}

impl<T> BorrowMut<T> for Changed<T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.data
    }
}

impl<T> Deref for Changed<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for Changed<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

/// A type that can get and set a changed flag
pub trait ChangedTrait {
    fn get_changed(&self) -> bool;
    fn set_changed(&self, dirty: bool);
}

impl<T> ChangedTrait for Changed<T> {
    fn get_changed(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }

    fn set_changed(&self, dirty: bool) {
        self.flag.store(dirty, Ordering::Relaxed);
    }
}

/// Construct implementation
impl<T> crate::Construct<T, crate::peano::Z> for Changed<T> {
    fn construct(t: T) -> Self {
        Changed {
            data: t,
            flag: false.into(),
        }
    }
}

impl<T, I, N> crate::Construct<T, crate::peano::S<I>> for Changed<N>
where
    N: crate::Construct<T, I>,
{
    fn construct(t: T) -> Self {
        Changed {
            data: N::construct(t),
            flag: false.into(),
        }
    }
}

/// With implementation
impl<T> crate::With<ChangedFlag, crate::peano::Z> for Changed<T> {
    fn with(self, t: ChangedFlag) -> Self {
        Changed {
            flag: t.0.into(),
            ..self
        }
    }
}

impl<T, I, N> crate::With<T, crate::peano::S<I>> for Changed<N>
where
    N: crate::With<T, I>,
{
    fn with(self, t: T) -> Self {
        Changed {
            data: self.data.with(t),
            ..self
        }
    }
}
