/// Utility trait for treating a struct as its own builder
pub trait With<T, I> {
    fn with(self, t: T) -> Self;
}

