use crate::{lexer::Token, parser::expression::{is_value, parse_value}};

mod expression;
use expression::Expression;

mod pattern;
use pattern::Pattern;

type PeekableLexer<'a> = std::iter::Peekable<logos::Lexer<'a, Token>>;

pub fn parse(lexer: &mut PeekableLexer<'_>) -> Result<Expression, ParsingError> {
    parse_expr(lexer, 0)
}

fn parse_expr(lexer: &mut PeekableLexer<'_>, min_bp: u8) -> Result<Expression, ParsingError> {
    // println!("Parsing expression: {:?}", lexer.peek());
    let mut lhs = match lexer.next() {
        Some(Ok(token)) => match token {
                t if is_value(&t) => parse_value(t),
                Token::SemiColon => return parse_expr(lexer, min_bp), // We just skip this call entirely
                Token::Let => parse_let(lexer)?,
                Token::If => parse_conditional(lexer)?,
                Token::LeftParenthesis => parse_parentheses(lexer)?,
                Token::Operator(op) if op == "-" => {
                    let (_, r_bp) = (0, 8);
                    let rhs = parse_expr(lexer, r_bp)?;
                    Expression::Operation(op, vec![rhs])
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
            Some(Ok(Token::SemiColon)) | Some(Ok(Token::RightParenthesis)) | Some(Ok(Token::Comma)) => break,

            Some(Ok(Token::Unit)) => {
                lexer.next();
                lhs = Expression::FunctionCall { 
                    callee: Box::new(lhs), 
                    argument: Box::new(Expression::Unit) 
                };
                break;
            }

            // This is for tuple variables
            Some(Ok(Token::LeftParenthesis)) => {
                let argument = Box::new(parse_expr(lexer, 0)?);
                dbg!(&lhs, &argument, lexer.peek());
                lhs = Expression::FunctionCall { callee: Box::new(lhs), argument };
                continue;
            }

            Some(Ok(Token::Operator(op))) => {
                let op = op.clone();
                if let Some((l_bp, r_bp)) = infix_binding_power(&op) {
                    if l_bp < min_bp {
                        break;
                    }
                    lexer.next();
                    let rhs = parse_expr(lexer, r_bp)?;
                    lhs = Expression::Operation(op, vec![lhs, rhs]);
                    continue;
                }
            },

            Some(Ok(Token::EqualSign)) => {
                if let Some((l_bp, r_bp)) = infix_binding_power("=") {
                    if l_bp < min_bp {
                        break;
                    }
                    lexer.next();
                    let rhs = parse_expr(lexer, r_bp)?;
                    lhs = Expression::Operation("=".to_owned(), vec![lhs, rhs]);
                    continue;
                }
                
            }

            Some(Ok(token)) if is_value(token) => {
                let argument = parse_value(lexer.next().unwrap().unwrap());
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
            Some(Ok(Token::Fun)) => declarations.push(parse_function(lexer, true)?),
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

fn parse_variable(lexer: &mut PeekableLexer<'_>) -> Result<Expression, ParsingError> {
    println!("Parsing variable: {:?}", lexer.peek());

    let name = match lexer.next() {
        Some(Ok(Token::Identifier(n))) => n,
        _ => return Err(ParsingError::InvalidSyntax),
    };

    assert_eq!(lexer.next(), Some(Ok(Token::EqualSign)));

    let value = Box::new(parse_expr(lexer, 0)?);

    Ok(Expression::VariableDefinition(name, value))
}

fn parse_function(lexer: &mut PeekableLexer<'_>, named: bool) -> Result<Expression, ParsingError> {
    println!("Parsing function: {:?}", lexer.peek());
    // We get the function's name if it is named
    let name = if named {
        match lexer.next() {
            Some(Ok(Token::Identifier(n))) => Some(n),
            _ => return Err(ParsingError::InvalidSyntax),
        }
    } else { None };

    let variable = match lexer.peek() {
        Some(Ok(Token::Identifier(_))) => {
            let Some(Ok(Token::Identifier(p))) = lexer.next() else { unreachable!() };
            Some(p)
        }
        Some(Ok(Token::Unit)) => {
            lexer.next();
            None
        }
        _ => return Err(ParsingError::InvalidSyntax)
    };

    match lexer.peek() {
        // We recursively descend if the function has more variables
        Some(Ok(Token::Identifier(_))) => {
            let body = Box::new(parse_function(lexer, false)?);
            Ok(Expression::Lambda(name, variable, body))
        }

        // Otherwise we simply compute the body
        Some(Ok(Token::EqualSign)) => {
            lexer.next();
            let body = Box::new(parse_expr(lexer, 0)?);
            Ok(Expression::Lambda(name, variable, body))
        }

        _ => Err(ParsingError::InvalidSyntax)
    }
}

fn infix_binding_power(operator: &str) -> Option<(u8, u8)> {
    match operator {
        "*" | "/" => Some((11, 12)),
        "+" | "-" => Some((9, 10)),
        "=" | "<=" | ">=" | "<" | ">" => Some((7, 8)),
        "andalso" => Some((5, 6)),
        "orelse" => Some((3, 4)),
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

fn parse_parentheses(lexer: &mut PeekableLexer<'_>) -> Result<Expression, ParsingError> {
    let lhs = parse_expr(lexer, 0)?;
    
    if lexer.peek() == Some(&Ok(Token::RightParenthesis)) {
        lexer.next();
        return Ok(lhs);
    }

    let mut expressions = vec![lhs];
    while matches!(lexer.peek(), Some(Ok(Token::Comma))) {
        lexer.next(); // We go past the comma
        expressions.push(parse_expr(lexer, 0)?);
    }

    assert_eq!(lexer.next(), Some(Ok(Token::RightParenthesis)));

    Ok(Expression::Tuple(expressions))
}

#[derive(Debug)]
pub enum ParsingError {
    InvalidSyntax
}