use std::{
    borrow::{Borrow, BorrowMut},
    marker::PhantomData,
    ops::{Deref, DerefMut},
};
pub use usage::{AsUsage, Usage};

/// Construct implementation
impl<U, T> crate::Construct<T, crate::peano::Z> for Usage<U, T> {
    fn construct(t: T) -> Self {
        U::as_usage(t)
    }
}

impl<T, I, U, N> crate::Construct<T, crate::peano::S<I>> for Usage<U, N>
where
    N: crate::Construct<T, I>,
{
    fn construct(t: T) -> Self {
        U::as_usage(N::construct(t))
    }
}

/// With implementation
impl<T, I, U, N> crate::With<T, crate::peano::S<I>> for Usage<U, N>
where
    N: crate::With<T, I>,
{
    fn with(self, t: T) -> Self {
        U::as_usage(self.data.with(t))
    }
}
