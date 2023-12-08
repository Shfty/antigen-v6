use crate::{Eval, EvalTrait};

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Mul<A, B = A>(pub A, pub B);
impl<C, A, B> EvalTrait<C> for Mul<A, B>
where
    A: EvalTrait<C>,
    B: EvalTrait<C>,
    Eval<A, C>: std::ops::Mul<Eval<B, C>>,
{
    type Eval = <Eval<A, C> as std::ops::Mul<Eval<B, C>>>::Output;

    fn eval(&self, ctx: &C) -> Self::Eval {
        self.0.eval(ctx) * self.1.eval(ctx)
    }
}
