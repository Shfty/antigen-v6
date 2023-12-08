use crate::{Expression, TokenExpression};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Token<'a> {
    Number(f32),
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Sin,
    Cos,
    Tan,
    Var(&'a str),
    OpenBracket,
    CloseBracket,
}

pub fn parse_expression(input: &str) -> Expression<f32> {
    // Parse tokens
    let (_, tokens) = parse_tokens(input).unwrap();

    // Convert tokens into token expressions
    let tokens = tokens
        .into_iter()
        .map(Into::<TokenExpression<f32>>::into)
        .map(|te| match te {
            TokenExpression::Token(t) => match t {
                Token::Number(n) => TokenExpression::Expression(Expression::Val(n)),
                Token::Var(v) => TokenExpression::Expression(Expression::Ident(v.to_string())),
                _ => TokenExpression::Token(t),
            },
            TokenExpression::Expression(_) => unreachable!(),
        })
        .collect::<Vec<_>>();

    parse_expression_impl(tokens)
}

pub fn parse_expression_impl<'a, 'b>(mut tokens: Vec<TokenExpression<'a, f32>>) -> Expression<f32> {
    println!("Tokens: {:#?}", tokens);

    // Recursively evalutate bracketed expressions
    while let Some(i) = tokens
        .iter()
        .position(|t| *t == TokenExpression::Token(Token::OpenBracket))
    {
        let mut sub_tokens = vec![];
        while i < tokens.len() {
            let token = tokens.remove(i);
            if token == TokenExpression::Token(Token::OpenBracket) {
                continue;
            }
            if token == TokenExpression::Token(Token::CloseBracket) {
                break;
            }
            sub_tokens.push(token)
        }

        let expression = parse_expression_impl(sub_tokens);
        tokens.insert(i, TokenExpression::Expression(expression));
    }

    // Parse functions
    parse_function(&mut tokens, Token::Sin, |val| Expression::Sin(val.into()));
    parse_function(&mut tokens, Token::Cos, |val| Expression::Cos(val.into()));
    parse_function(&mut tokens, Token::Tan, |val| Expression::Tan(val.into()));

    // Convert TokenExpression::Token into TokenExpression::Expression
    parse_operator(&mut tokens, Token::Pow, |lhs, rhs| {
        Expression::Pow(lhs.into(), rhs.into())
    });
    parse_operator(&mut tokens, Token::Div, |lhs, rhs| {
        Expression::Div(lhs.into(), rhs.into())
    });
    parse_operator(&mut tokens, Token::Mul, |lhs, rhs| {
        Expression::Mul(lhs.into(), rhs.into())
    });
    parse_operator(&mut tokens, Token::Add, |lhs, rhs| {
        Expression::Add(lhs.into(), rhs.into())
    });
    parse_operator(&mut tokens, Token::Sub, |lhs, rhs| {
        Expression::Sub(lhs.into(), rhs.into())
    });

    // Convert TokenExpression::Expression into Expression
    tokens
        .into_iter()
        .map(|ts| match ts {
            TokenExpression::Expression(e) => e,
            _ => panic!("Failed to parse token"),
        })
        .next()
        .unwrap()
}

fn parse_function<'a, V, F>(
    tokens: &mut Vec<TokenExpression<V>>,
    op_token: Token<'a>,
    func_cons: F,
) where
    V: PartialEq,
    F: Fn(Expression<V>) -> Expression<V>,
{
    while let Some(i) = tokens
        .iter()
        .position(|t| *t == TokenExpression::Token(op_token))
    {
        match tokens.remove(i) {
            TokenExpression::Token(op_token) => TokenExpression::<V>::Token(op_token),
            _ => panic!("Unexpected Function"),
        };
        let val = match tokens.remove(i) {
            TokenExpression::Expression(e) => e,
            _ => panic!("Unexpected {:?} Parameter", op_token),
        };
        tokens.insert(i, TokenExpression::Expression(func_cons(val)));
    }
}

fn parse_operator<'a, V, F>(
    tokens: &mut Vec<TokenExpression<V>>,
    op_token: Token<'a>,
    expr_cons: F,
) where
    V: PartialEq,
    F: Fn(Expression<V>, Expression<V>) -> Expression<V>,
{
    while let Some(i) = tokens
        .iter()
        .position(|t| *t == TokenExpression::Token(op_token))
    {
        let lhs = match tokens.remove(i - 1) {
            TokenExpression::Expression(e) => e,
            _ => panic!("Unexpected {:?} LHS", op_token),
        };
        match tokens.remove(i - 1) {
            TokenExpression::Token(op_token) => TokenExpression::<V>::Token(op_token),
            _ => panic!("Unexpected Operator"),
        };
        let rhs = match tokens.remove(i - 1) {
            TokenExpression::Expression(e) => e,
            _ => panic!("Unexpected {:?} RHS", op_token),
        };
        tokens.insert(i - 1, TokenExpression::Expression(expr_cons(lhs, rhs)));
    }
}

fn parse_tokens(input: &str) -> nom::IResult<&str, Vec<Token>> {
    nom::multi::fold_many1(parse_token, Vec::new, |mut acc: Vec<_>, item| {
        acc.push(item);
        acc
    })(input)
}

fn parse_token(input: &str) -> nom::IResult<&str, Token> {
    nom::branch::alt((
        parse_open_bracket,
        parse_close_bracket,
        parse_pow,
        parse_div,
        parse_mul,
        parse_add,
        parse_sub,
        parse_number,
        parse_sin,
        parse_cos,
        parse_tan,
        parse_var,
    ))(input)
}

fn parse_number(input: &str) -> nom::IResult<&str, Token> {
    let (input, output) = whitespaced(nom::number::complete::float)(input)?;
    Ok((input, Token::Number(output)))
}

fn parse_open_bracket(input: &str) -> nom::IResult<&str, Token> {
    let (input, _) = ws_char('(')(input)?;
    Ok((input, Token::OpenBracket))
}

fn parse_close_bracket(input: &str) -> nom::IResult<&str, Token> {
    let (input, _) = ws_char(')')(input)?;
    Ok((input, Token::CloseBracket))
}

fn parse_add(input: &str) -> nom::IResult<&str, Token> {
    let (input, _) = ws_char('+')(input)?;
    Ok((input, Token::Add))
}

fn parse_sub(input: &str) -> nom::IResult<&str, Token> {
    let (input, _) = ws_char('-')(input)?;
    Ok((input, Token::Sub))
}

fn parse_mul(input: &str) -> nom::IResult<&str, Token> {
    let (input, _) = ws_char('*')(input)?;
    Ok((input, Token::Mul))
}

fn parse_div(input: &str) -> nom::IResult<&str, Token> {
    let (input, _) = ws_char('/')(input)?;
    Ok((input, Token::Div))
}

fn parse_pow(input: &str) -> nom::IResult<&str, Token> {
    let (input, _) = ws_char('^')(input)?;
    Ok((input, Token::Pow))
}

fn parse_sin(input: &str) -> nom::IResult<&str, Token> {
    let (input, _) = whitespaced(nom::bytes::complete::tag("sin"))(input)?;
    Ok((input, Token::Sin))
}

fn parse_cos(input: &str) -> nom::IResult<&str, Token> {
    let (input, _) = whitespaced(nom::bytes::complete::tag("cos"))(input)?;
    Ok((input, Token::Cos))
}

fn parse_tan(input: &str) -> nom::IResult<&str, Token> {
    let (input, _) = whitespaced(nom::bytes::complete::tag("tan"))(input)?;
    Ok((input, Token::Tan))
}

fn parse_var(input: &str) -> nom::IResult<&str, Token> {
    let (input, output) = whitespaced(nom::character::complete::alpha1)(input)?;
    Ok((input, Token::Var(output)))
}

fn whitespaced<'a, F: 'a, O, E: nom::error::ParseError<&'a str>>(
    inner: F,
) -> impl FnMut(&'a str) -> nom::IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> nom::IResult<&'a str, O, E>,
{
    nom::sequence::delimited(
        nom::character::complete::multispace0,
        inner,
        nom::character::complete::multispace0,
    )
}

fn ws_char<'a, E: nom::error::ParseError<&'a str> + 'a>(
    c: char,
) -> impl FnMut(&'a str) -> nom::IResult<&'a str, char, E> {
    move |input| whitespaced(nom::character::complete::char(c))(input)
}
