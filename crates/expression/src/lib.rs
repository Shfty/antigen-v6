mod eval;
mod op_types;
mod expression;
mod parse;

pub use eval::*;
pub use op_types::*;
pub use expression::*;
pub use parse::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_expression() {
        let vars = [("x", 2.0), ("y", 4.0), ("z", 6.0)]
            .into_iter()
            .collect::<BTreeMap<_, _>>();

        println!(
            "Result: {}",
            Sub(Add(1.0, Var("x")), Div(Mul(3.0, Var("y")), 5.0)).eval(&vars)
        );

        println!(
            "Result: {}",
            Expression::Sub(
                Expression::Add(Expression::Val(1.0).into(), Expression::Ident("x").into()).into(),
                Expression::Div(
                    Expression::Mul(Expression::Val(3.0).into(), Expression::Ident("y").into()).into(),
                    Expression::Val(5.0).into(),
                )
                .into(),
            )
            .eval(&vars),
        );

        let expression = parse_expression("sin(1 + x) - cos(3 * y) / tan(5 ^ z)");
        println!("Expression: {:#?}", expression);
        println!("Result: {}", expression.eval(&vars));
    }
}
