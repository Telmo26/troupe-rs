use crate::lexer::Token;

mod ast;
use ast::{AST, Pattern};

mod utils;
use utils::{is_value, parse_value};

type PeekableLexer<'a> = std::iter::Peekable<logos::Lexer<'a, Token>>;

pub fn parse(lexer: &mut PeekableLexer<'_>) -> Result<AST, ParsingError> {
    parse_expr(lexer, 0)
}

fn parse_expr(lexer: &mut PeekableLexer<'_>, min_bp: u8) -> Result<AST, ParsingError> {
    println!("Parsing expression: {:?}", lexer.peek());
    let mut lhs = match lexer.next() {
        Some(Ok(token)) => match token {
                t if is_value(&t) => parse_value(t),
                Token::Wildcard => return Ok(AST::Wildcard),
                Token::In => return parse_expr(lexer, min_bp), // We just skip this call entirely
                Token::Let => parse_let(lexer)?,
                Token::Val => {
                    let (name, value) = parse_variable(lexer)?;
                    parse_declaration(lexer, name, value)?
                },
                Token::Fun => {
                    let (name, value) = parse_function(lexer, true)?;
                    parse_declaration(lexer, name, value)?
                },
                Token::Case => parse_case(lexer)?,
                Token::If => parse_conditional(lexer)?,
                Token::LeftParenthesis => parse_parenthesis(lexer)?,
                Token::LeftBracket => parse_list(lexer)?,
                Token::Operator(op) if op == "-" => {
                    let (_, r_bp) = (0, 8);
                    let rhs = parse_expr(lexer, r_bp)?;
                    AST::Operation(op, vec![rhs])
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
                lhs = AST::Let { 
                    name: Pattern::Empty, 
                    value: Box::new(lhs), 
                    body: Box::new(parse_expr(lexer, 0)?)
                };
                break;
            } 
            
            Some(Ok(Token::RightParenthesis)) 
            | Some(Ok(Token::RightBracket))
            | Some(Ok(Token::Comma)) 
            | Some(Ok(Token::Disjunction)) => break,

            // For a function unit must be its only parameter
            Some(Ok(Token::Unit)) => {
                lexer.next();
                lhs = AST::FunctionCall { 
                    callee: Box::new(lhs), 
                    argument: Box::new(AST::Unit) 
                };
                break;
            }

            // This is for tuple or list function parameters
            Some(Ok(Token::LeftParenthesis)) | Some(Ok(Token::LeftBracket)) => {
                let argument = Box::new(parse_expr(lexer, FUNCTION_CALL_BINDING_POWER)?);
                lhs = AST::FunctionCall { callee: Box::new(lhs), argument };
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
                    lhs = AST::Operation(op, vec![lhs, rhs]);
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
                    lhs = AST::Operation("=".to_owned(), vec![lhs, rhs]);
                    continue;
                }
                
            }

            Some(Ok(token)) if is_value(token) => {
                let argument = parse_value(lexer.next().unwrap().unwrap());
                lhs = AST::FunctionCall { callee: Box::new(lhs), argument: Box::new(argument) };
                continue;
            },
            
            _ => break,
        };

    }

    Ok(lhs)
}

fn parse_let(lexer: &mut PeekableLexer<'_>) -> Result<AST, ParsingError> {
    let (name, value) = match lexer.next() {
        Some(Ok(Token::Val)) => parse_variable(lexer)?,
        Some(Ok(Token::Fun)) => parse_function(lexer, true)?,
        _ => return Err(ParsingError::InvalidSyntax)
    };

    parse_declaration(lexer, name, value)
}

fn parse_declaration(lexer: &mut PeekableLexer<'_>, name: Pattern, value: AST) -> Result<AST, ParsingError> {
    let body = Box::new(parse_expr(lexer, 0)?);

    Ok(AST::Let { 
        name, 
        value: Box::new(value), 
        body,
    })
}

fn parse_variable(lexer: &mut PeekableLexer<'_>) -> Result<(Pattern, AST), ParsingError> {
    println!("Parsing variable: {:?}", lexer.peek());

    match lexer.next() {
        // This is the normal assignment
        Some(Ok(Token::Identifier(name))) =>{
            assert_eq!(lexer.next(), Some(Ok(Token::EqualSign)));

            let value = parse_expr(lexer, 0)?;

            Ok((Pattern::Single(Box::new(AST::Identifier(name))), value))
        },

        // This is pattern-matching
        Some(Ok(Token::LeftParenthesis)) => {
            let mut expressions = vec![parse_expr(lexer, 0)?];
            while matches!(lexer.peek(), Some(Ok(Token::Comma))) {
                lexer.next();
                expressions.push(parse_expr(lexer, 0)?);
            }

            assert_eq!(lexer.next(), Some(Ok(Token::RightParenthesis)));
            assert_eq!(lexer.next(), Some(Ok(Token::EqualSign)));

            let value = parse_expr(lexer, 0)?;

            Ok((Pattern::Tuple(expressions), value))
        }
        _ => return Err(ParsingError::InvalidSyntax),
    }    
}

fn parse_function(lexer: &mut PeekableLexer<'_>, named: bool) -> Result<(Pattern, AST), ParsingError> {
    println!("Parsing function: {:?}", lexer.peek());
    // We get the function's name if it is named
    let name = if named {
        match lexer.next() {
            Some(Ok(Token::Identifier(n))) => Pattern::Single(Box::new(AST::Identifier(n))),
            Some(Ok(Token::Wildcard)) => Pattern::Empty,
            _ => return Err(ParsingError::InvalidSyntax),
        }
    } else { Pattern::Empty };

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
            let (_, body) = parse_function(lexer, false)?;
            Ok((name, AST::Lambda(variable, Box::new(body))))
        }

        // Otherwise we simply compute the body
        Some(Ok(Token::EqualSign)) => {
            lexer.next();
            let body = Box::new(parse_expr(lexer, 0)?);
            Ok((name, AST::Lambda(variable, body)))
        }

        _ => return Err(ParsingError::InvalidSyntax)
    }
}

const FUNCTION_CALL_BINDING_POWER: u8 = 13;
fn infix_binding_power(operator: &str) -> Option<(u8, u8)> {
    match operator {
        "*" | "/" => Some((11, 12)),
        "+" | "-" => Some((9, 10)),
        "=" | "<=" | ">=" | "<" | ">" => Some((7, 8)),
        "andalso" => Some((5, 6)),
        "orelse" => Some((3, 4)),
        "raisedTo" => Some((1, 2)),
        _ => None
    }
}

fn parse_conditional(lexer: &mut PeekableLexer<'_>) -> Result<AST, ParsingError> {
    let condition = Box::new(parse_expr(lexer, 0)?);

    assert_eq!(lexer.next(), Some(Ok(Token::Then)));

    let first_path = Box::new(parse_expr(lexer, 0)?);

    let mut second_path = None;
    if matches!(lexer.peek(), Some(Ok(Token::Else))) {
        lexer.next();
        second_path = Some(Box::new(parse_expr(lexer, 0)?));
    };

    Ok(AST::Conditional(condition, first_path, second_path))
}

fn parse_parenthesis(lexer: &mut PeekableLexer<'_>) -> Result<AST, ParsingError> {
    let lhs = parse_expr(lexer, 0)?;

    if lexer.peek() == Some(&Ok(Token::RightParenthesis)) {
        lexer.next();
        return Ok(lhs)
    }

    let mut expressions = vec![lhs];
    while matches!(lexer.peek(), Some(Ok(Token::Comma))) {
        lexer.next(); // We go past the comma
        expressions.push(parse_expr(lexer, 0)?);
    }

    assert_eq!(lexer.next(), Some(Ok(Token::RightParenthesis)));

    Ok(AST::Tuple(expressions))
}

fn parse_case(lexer: &mut PeekableLexer<'_>) -> Result<AST, ParsingError> {
    let tested_expression = Box::new(parse_expr(lexer, 0)?);

    assert_eq!(lexer.next(), Some(Ok(Token::Of)));

    let mut patterns = vec![];
    loop {
        let pattern = match lexer.peek() {
            Some(Ok(token)) => match token {
                t if is_value(t) => {
                    let value = lexer.next().unwrap().unwrap();
                    Pattern::Single(Box::new(parse_value(value)))
                }
                Token::LeftParenthesis => {
                    lexer.next();
                    let mut expressions = vec![parse_expr(lexer, 0)?];
                    while matches!(lexer.peek(), Some(Ok(Token::Comma))) {
                        lexer.next(); // We go past the comma
                        expressions.push(parse_expr(lexer, 0)?);
                    }
                    assert_eq!(lexer.next(), Some(Ok(Token::RightParenthesis)));
                    Pattern::Tuple(expressions)
                }
                _ => return Err(ParsingError::InvalidSyntax)
            }
            _ => return Err(ParsingError::InvalidSyntax)
        };

        assert_eq!(lexer.next(), Some(Ok(Token::Arrow)));

        let expression = parse_expr(lexer, 0)?;

        patterns.push((pattern, expression));

        if matches!(lexer.peek(), Some(Ok(Token::Disjunction))) {
            lexer.next(); // We go past the disjunction
        } else {
            break;
        }
    };

    patterns.reverse();

    let mut previous_turn = AST::Unit;
    for (pattern, expression) in patterns {
        previous_turn = AST::Conditional(
            Box::new(AST::Match(pattern, tested_expression.clone())), 
            Box::new(expression), 
            if matches!(previous_turn, AST::Unit) { None } else { Some(Box::new(previous_turn)) }
        );
    };

    Ok(previous_turn)
}

fn parse_list(lexer: &mut PeekableLexer<'_>) -> Result<AST, ParsingError> {
    let mut expressions = vec![parse_expr(lexer, 0)?];

    while matches!(lexer.peek(), Some(Ok(Token::Comma))) {
        lexer.next();
        expressions.push(parse_expr(lexer, 0)?);
    };

    assert_eq!(lexer.next(), Some(Ok(Token::RightBracket)));

    Ok(AST::List(expressions))
}

#[derive(Debug)]
pub enum ParsingError {
    InvalidSyntax
}