/// A lazily-initialized component that can be pending, ready, or dropped
#[derive(Debug)]
pub enum LazyComponent<T> {
    Pending,
    Ready(T),
    Dropped,
}

impl<T> Default for LazyComponent<T> {
    fn default() -> Self {
        LazyComponent::Pending
    }
}

impl<T> LazyComponent<T> {
    pub fn is_pending(&self) -> bool {
        matches!(self, LazyComponent::Pending)
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, LazyComponent::Ready(_))
    }

    pub fn is_dropped(&self) -> bool {
        matches!(self, LazyComponent::Dropped)
    }

    pub fn set_pending(&mut self) {
        *self = LazyComponent::Pending;
    }

    pub fn set_ready(&mut self, inner: T) {
        *self = LazyComponent::Ready(inner);
    }

    pub fn set_dropped(&mut self) {
        *self = LazyComponent::Dropped;
    }

    pub fn take(&mut self) -> Self {
        std::mem::replace(self, LazyComponent::Dropped)
    }

    pub fn get(&self) -> Option<&T> {
        match self {
            LazyComponent::Ready(component) => Some(component),
            _ => None,
        }
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        match self {
            LazyComponent::Ready(component) => Some(component),
            _ => None,
        }
    }
}
