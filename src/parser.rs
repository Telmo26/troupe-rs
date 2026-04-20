use crate::{lexer::Token, parser::expression::Declaration};

mod expression;
use expression::Expression;

type PeekableLexer<'a> = std::iter::Peekable<logos::Lexer<'a, Token>>;

pub fn parse(lexer: &mut PeekableLexer<'_>) -> Result<Expression, ParsingError> {
    parse_expr(lexer, 0)
}

fn parse_expr(lexer: &mut PeekableLexer<'_>, min_bp: u8) -> Result<Expression, ParsingError> {
    // println!("Parsing expression: {:?}", lexer.peek());
    let mut lhs = match lexer.next() {
        Some(Ok(token)) => match token {
                Token::Let => parse_let(lexer)?,
                Token::Number(n) => Expression::Number(n),
                Token::StringLiteral(s) => Expression::StringLiteral(s),
                Token::Boolean(b) => Expression::Boolean(b),
                Token::Identifier(s) => Expression::Identifier(s),
                Token::If => parse_conditional(lexer)?,
                Token::MathOperator(op) => if op == "(" {
                    let lhs = parse_expr(lexer, 0)?;
                    assert_eq!(lexer.next(), Some(Ok(Token::MathOperator(")".to_owned()))));
                    lhs
                } else {
                    let (_, r_bp) = (0, 8);
                    let rhs = parse_expr(lexer, r_bp)?;
                    Expression::MathOperation(op, vec![rhs])
                }
                _ => {
                    println!("Parsing {token:?}");
                    return Err(ParsingError::InvalidSyntax)
                }
        }
        _ => return Err(ParsingError::InvalidSyntax),
    };

    loop {
        match lexer.peek() {
            Some(Ok(Token::SemiColon)) => {
                lexer.next();
                break;
            },

            Some(Ok(Token::MathOperator(op))) if op == "(" => {
                lexer.next();
                let argument = Box::new(parse_expr(lexer, 0)?);
                assert_eq!(lexer.next(), Some(Ok(Token::MathOperator(")".to_owned()))));
                lhs = Expression::FunctionCall { callee: Box::new(lhs), argument };
                continue;
            }

            Some(Ok(Token::MathOperator(op))) if op == ")" => break,

            Some(Ok(Token::MathOperator(op))) => {
                let op = op.clone();
                if let Some((l_bp, r_bp)) = infix_binding_power(&op) {
                    if l_bp < min_bp {
                        break;
                    }
                    lexer.next();
                    lhs = {
                        let rhs = parse_expr(lexer, r_bp)?;
                        Expression::MathOperation(op, vec![lhs, rhs])
                    };
                    continue;
                }
            },

            Some(Ok(Token::BooleanOperator(op))) => {
                let op = op.clone();
                lexer.next();
                lhs = {
                    let rhs = parse_expr(lexer, 0)?;
                    Expression::BooleanOperation(op, vec![lhs, rhs])
                };
                continue;
            },

            Some(Ok(Token::EqualSign)) => {
                lexer.next();
                let rhs = parse_expr(lexer, 0)?;
                lhs = Expression::BooleanOperation("=".to_owned(), vec![lhs, rhs]);
                continue;
            }

            Some(Ok(Token::Number(_))) 
            | Some(Ok(Token::StringLiteral(_))) 
            | Some(Ok(Token::Identifier(_))) => {
                let argument = match lexer.next().unwrap().unwrap() {
                    Token::Number(n) => Expression::Number(n),
                    Token::StringLiteral(s) => Expression::StringLiteral(s),
                    Token::Identifier(i) => Expression::Identifier(i),
                    _ => unreachable!(),
                };
                lhs = Expression::FunctionCall { callee: Box::new(lhs), argument: Box::new(argument) };
                continue;
            },
            
            _ => break,
        };

    }

    Ok(lhs)
}

fn parse_let(lexer: &mut PeekableLexer<'_>) -> Result<Expression, ParsingError> {
    let mut declarations = Vec::new();
    while matches!(lexer.peek(), Some(Ok(Token::Val)) | Some(Ok(Token::Fun))) {
        match lexer.next() {
            Some(Ok(Token::Val)) => declarations.push(parse_variable(lexer)?),
            Some(Ok(Token::Fun)) => declarations.push(parse_function(lexer)?),
            _ => unreachable!()
        }
        
    }

    assert_eq!(lexer.next(), Some(Ok(Token::In)));

    let mut body = Vec::new();
    while !matches!(lexer.peek(), Some(Ok(Token::End))) {
        body.push(parse_expr(lexer, 0)?);
    }

    assert_eq!(lexer.next(), Some(Ok(Token::End)));

    Ok(Expression::Let { declarations, body })
}

fn parse_variable(lexer: &mut PeekableLexer<'_>) -> Result<Declaration, ParsingError> {
    let name = match lexer.next() {
        Some(Ok(Token::Identifier(n))) => n,
        _ => return Err(ParsingError::InvalidSyntax),
    };

    assert_eq!(lexer.next(), Some(Ok(Token::EqualSign)));

    let value = parse_expr(lexer, 0)?;

    Ok(Declaration::Variable {
        name,
        value
    })
}

fn parse_function(lexer: &mut PeekableLexer<'_>) -> Result<Declaration, ParsingError> {
    let name = match lexer.next() {
        Some(Ok(Token::Identifier(n))) => n,
        _ => return Err(ParsingError::InvalidSyntax),
    };

    let mut parameters = Vec::new();
    while let Some(Ok(Token::Identifier(_))) = lexer.peek() 
        && let Some(Ok(Token::Identifier(s))) = lexer.next() 
    {
        parameters.push(s);
    };

    assert_eq!(lexer.next(), Some(Ok(Token::EqualSign)));

    let body = parse_expr(lexer, 0)?;

    Ok(Declaration::Function { name, parameters, body })
}

fn infix_binding_power(operator: &str) -> Option<(u8, u8)> {
    match operator {
        "+" | "-" => Some((5, 6)),
        "*" | "/" => Some((7, 8)),
        _ => None
    }
}

fn parse_conditional(lexer: &mut PeekableLexer<'_>) -> Result<Expression, ParsingError> {
    let condition = Box::new(parse_expr(lexer, 0)?);

    assert_eq!(lexer.next(), Some(Ok(Token::Then)));

    let first_path = Box::new(parse_expr(lexer, 0)?);

    let mut second_path = None;
    if matches!(lexer.peek(), Some(Ok(Token::Else))) {
        lexer.next();
        second_path = Some(Box::new(parse_expr(lexer, 0)?));
    };

    Ok(Expression::Conditional(condition, first_path, second_path))
}

#[derive(Debug)]
pub enum ParsingError {
    InvalidSyntax
}