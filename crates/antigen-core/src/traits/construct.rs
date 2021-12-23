// Utility trait for constructing nested newtypes
pub trait Construct<T, I> {
    fn construct(t: T) -> Self;
}

impl<T> Construct <T, crate::peano::Z> for std::sync::Arc<T> {
    fn construct(t: T) -> Self {
        std::sync::Arc::new(t)
    }
}
