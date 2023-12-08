use crate::{EvalTrait, Token};

#[derive(Debug, Clone, PartialEq)]
pub enum Expression<V> {
    Val(V),
    Ident(String),
    Add(Box<Expression<V>>, Box<Expression<V>>),
    Sub(Box<Expression<V>>, Box<Expression<V>>),
    Mul(Box<Expression<V>>, Box<Expression<V>>),
    Div(Box<Expression<V>>, Box<Expression<V>>),
    Pow(Box<Expression<V>>, Box<Expression<V>>),
    Sin(Box<Expression<V>>),
    Cos(Box<Expression<V>>),
    Tan(Box<Expression<V>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenExpression<'a, V> {
    Token(Token<'a>),
    Expression(Expression<V>),
}

impl<'a, V> From<Token<'a>> for TokenExpression<'a, V> {
    fn from(t: Token<'a>) -> Self {
        TokenExpression::Token(t)
    }
}

impl<'a, V> From<Expression<V>> for TokenExpression<'a, V> {
    fn from(e: Expression<V>) -> Self {
        TokenExpression::Expression(e)
    }
}

impl EvalTrait<std::collections::BTreeMap<&str, u32>> for Expression<u32>
{
    type Eval = u32;

    fn eval(&self, ctx: &std::collections::BTreeMap<&str, u32>) -> Self::Eval {
        match self {
            Expression::Add(lhs, rhs) => (*lhs).eval(ctx) + (*rhs).eval(ctx),
            Expression::Sub(lhs, rhs) => (*lhs).eval(ctx) - (*rhs).eval(ctx),
            Expression::Mul(lhs, rhs) => (*lhs).eval(ctx) * (*rhs).eval(ctx),
            Expression::Div(lhs, rhs) => (*lhs).eval(ctx) / (*rhs).eval(ctx),
            Expression::Pow(lhs, rhs) => (*lhs).eval(ctx).pow((*rhs).eval(ctx)),
            Expression::Sin(_) => panic!("No sine function for u32"),
            Expression::Cos(_) => panic!("No cosine function for u32"),
            Expression::Tan(_) => panic!("No tangent function for u32"),
            Expression::Val(n) => *n,
            Expression::Ident(k) => ctx[k.as_str()],
        }
    }
}

impl EvalTrait<std::collections::BTreeMap<&str, f32>> for Expression<f32>
{
    type Eval = f32;

    fn eval(&self, ctx: &std::collections::BTreeMap<&str, f32>) -> Self::Eval {
        match self {
            Expression::Add(lhs, rhs) => (*lhs).eval(ctx) + (*rhs).eval(ctx),
            Expression::Sub(lhs, rhs) => (*lhs).eval(ctx) - (*rhs).eval(ctx),
            Expression::Mul(lhs, rhs) => (*lhs).eval(ctx) * (*rhs).eval(ctx),
            Expression::Div(lhs, rhs) => (*lhs).eval(ctx) / (*rhs).eval(ctx),
            Expression::Pow(lhs, rhs) => (*lhs).eval(ctx).powf((*rhs).eval(ctx)),
            Expression::Sin(val) => (*val).eval(ctx).sin(),
            Expression::Cos(val) => (*val).eval(ctx).cos(),
            Expression::Tan(val) => (*val).eval(ctx).tan(),
            Expression::Val(n) => *n,
            Expression::Ident(k) => ctx[k.as_str()],
        }
    }
}
