pub trait EvalTrait<C> {
    type Eval;

    fn eval(&self, ctx: &C) -> Self::Eval;
}

pub type Eval<T, C> = <T as EvalTrait<C>>::Eval;

impl<C> EvalTrait<C> for u32 {
    type Eval = u32;

    fn eval(&self, _: &C) -> Self::Eval {
        *self
    }
}

impl<C> EvalTrait<C> for i32 {
    type Eval = i32;

    fn eval(&self, _: &C) -> Self::Eval {
        *self
    }
}

impl<C> EvalTrait<C> for f32 {
    type Eval = f32;

    fn eval(&self, _: &C) -> Self::Eval {
        *self
    }
}
