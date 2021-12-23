//! Peano encoding for natural numbers
use std::marker::PhantomData;

pub struct Z;
pub struct S<N>(PhantomData<N>);
