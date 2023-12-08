use crate::EvalTrait;

#[derive(Debug, Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Var<K>(pub K);

impl<C, K, V> EvalTrait<C> for Var<K>
where
    C: std::ops::Index<K, Output = V>,
    K: Copy,
    V: Copy,
{
    type Eval = V;

    fn eval(&self, ctx: &C) -> Self::Eval {
        ctx[self.0]
    }
}

