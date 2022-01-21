use crate::{Construct, peano::Z};

/// A lazily-initialized component that can be pending, ready, or dropped
#[derive(Debug)]
pub enum LazyComponent<R, P = (), D = ()> {
    Pending(P),
    Ready(R),
    Dropped(D),
}

impl<R, P, D> Default for LazyComponent<R, P, D>
where
    P: Default,
{
    fn default() -> Self {
        LazyComponent::Pending(Default::default())
    }
}

impl<R, P, D> Clone for LazyComponent<R, P, D>
where
    R: Clone,
    P: Clone,
    D: Clone,
{
    fn clone(&self) -> Self {
        match self {
            LazyComponent::Pending(p) => LazyComponent::Pending(p.clone()),
            LazyComponent::Ready(r) => LazyComponent::Ready(r.clone()),
            LazyComponent::Dropped(d) => LazyComponent::Dropped(d.clone()),
        }
    }
}

impl<R, P, D> Copy for LazyComponent<R, P, D>
where
    R: Copy,
    P: Copy,
    D: Copy,
{
}

impl<R, P, D> PartialEq for LazyComponent<R, P, D>
where
    R: PartialEq,
    P: PartialEq,
    D: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LazyComponent::Pending(lhs), LazyComponent::Pending(rhs)) => lhs.eq(rhs),
            (LazyComponent::Ready(lhs), LazyComponent::Ready(rhs)) => lhs.eq(rhs),
            (LazyComponent::Dropped(lhs), LazyComponent::Dropped(rhs)) => lhs.eq(rhs),
            _ => false,
        }
    }
}

impl<R, P, D> Eq for LazyComponent<R, P, D>
where
    R: Eq,
    P: Eq,
    D: Eq,
{
}

impl<R, P, D> LazyComponent<R, P, D> {
    pub fn new(p: P) -> LazyComponent<R, P, D> {
        LazyComponent::Pending(p)
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, LazyComponent::Pending(_))
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, LazyComponent::Ready(_))
    }

    pub fn is_dropped(&self) -> bool {
        matches!(self, LazyComponent::Dropped(_))
    }

    pub fn set_pending(&mut self)
    where
        P: Default,
    {
        *self = LazyComponent::Pending(Default::default());
    }

    pub fn set_pending_with(&mut self, p: P) {
        *self = LazyComponent::Pending(p);
    }

    pub fn set_ready(&mut self)
    where
        R: Default,
    {
        *self = LazyComponent::Ready(Default::default());
    }

    pub fn set_ready_with(&mut self, r: R) {
        *self = LazyComponent::Ready(r);
    }

    pub fn set_dropped(&mut self)
    where
        D: Default,
    {
        *self = LazyComponent::Dropped(Default::default());
    }

    pub fn set_dropped_with(&mut self, d: D) {
        *self = LazyComponent::Dropped(d);
    }

    pub fn take(&mut self) -> Self
    where
        D: Default,
    {
        std::mem::replace(self, LazyComponent::Dropped(Default::default()))
    }

    pub fn get(&self) -> Option<&R> {
        match self {
            LazyComponent::Ready(r) => Some(r),
            _ => None,
        }
    }

    pub fn get_mut(&mut self) -> Option<&mut R> {
        match self {
            LazyComponent::Ready(r) => Some(r),
            _ => None,
        }
    }
}

impl<R, P, D> Construct<P, Z> for LazyComponent<R, P, D> {
    fn construct(r: P) -> Self {
        LazyComponent::Pending(r)
    }
}

