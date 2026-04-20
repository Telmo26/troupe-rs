use crate::lexer::Token;

mod expression;
use expression::Expression;

type PeekableLexer<'a> = std::iter::Peekable<logos::Lexer<'a, Token>>;

pub fn parse(lexer: &mut PeekableLexer<'_>) -> Result<Expression, ParsingError> {
    parse_expr(lexer, 0)
}

fn parse_expr(lexer: &mut PeekableLexer<'_>, min_bp: u8) -> Result<Expression, ParsingError> {
    println!("Parsing expression: {:?}", lexer.peek());
    let mut lhs = match lexer.next() {
        Some(Ok(token)) => match token {
                Token::Let => parse_let(lexer)?,
                Token::Number(n) => Expression::Number(n),
                Token::StringLiteral(s) => Expression::StringLiteral(s),
                Token::Boolean(b) => Expression::Boolean(b),
                Token::Identifier(s) => Expression::Identifier(s),
                Token::If => parse_conditional(lexer)?,
                Token::Operator(op) => match (op.as_str(), lexer.peek()) {
                    ("(", Some(Ok(Token::Operator(op)))) if op == ")" => {
                        lexer.next();
                        Expression::Unit
                    }
                    ("(", _) => {
                        let lhs = parse_expr(lexer, 0)?;
                        assert_eq!(lexer.next(), Some(Ok(Token::Operator(")".to_owned()))));
                        lhs
                    },
                    (_, _) => {
                        let (_, r_bp) = (0, 8);
                        let rhs = parse_expr(lexer, r_bp)?;
                        Expression::Operation(op, vec![rhs])
                    }
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

            Some(Ok(Token::Operator(op))) if op == "(" => {
                let argument = Box::new(parse_expr(lexer, 0)?);
                lhs = Expression::FunctionCall { callee: Box::new(lhs), argument };
                continue;
            }

            Some(Ok(Token::Operator(op))) if op == ")" => break,

            Some(Ok(Token::Operator(op))) => {
                let op = op.clone();
                if let Some((l_bp, r_bp)) = infix_binding_power(&op) {
                    if l_bp < min_bp {
                        break;
                    }
                    lexer.next();
                    lhs = {
                        let rhs = parse_expr(lexer, r_bp)?;
                        Expression::Operation(op, vec![lhs, rhs])
                    };
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
    
    let first_argument = match lexer.peek() {
        Some(Ok(Token::Identifier(_))) => {
            let Token::Identifier(p) = lexer.next().unwrap().unwrap() else { unreachable!() };
            Some(p)
        }
        Some(Ok(Token::Operator(op))) if op == "(" => {
            lexer.next();
            // Empty argument
            if lexer.peek() == Some(&Ok(Token::Operator(")".to_string()))) {
                lexer.next();
                None
            } else { // Tuple argument
                todo!("Implement tuples")
            }
        }
        _ => return Err(ParsingError::InvalidSyntax)
    };

    match lexer.peek() {
        // We recursively descend if the function has more arguments
        Some(Ok(Token::Identifier(_))) => {
            let body = Box::new(parse_function(lexer, false)?);
            Ok(Expression::Lambda(name, first_argument, body))
        }

        // Otherwise we simply compute the body
        Some(Ok(Token::EqualSign)) => {
            lexer.next();
            let body = Box::new(parse_expr(lexer, 0)?);
            Ok(Expression::Lambda(name, first_argument, body))
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

#[derive(Debug)]
pub enum ParsingError {
    InvalidSyntax
}