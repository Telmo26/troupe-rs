use crate::{lexer::Token, parser::expression::Declaration};

mod expression;
use expression::Expression;

type PeekableLexer<'a> = std::iter::Peekable<logos::Lexer<'a, Token>>;

pub fn parse(lexer: &mut PeekableLexer<'_>) -> Result<Expression, ParsingError> {
    parse_expr(lexer)
}

fn parse_expr(lexer: &mut PeekableLexer<'_>) -> Result<Expression, ParsingError> {
    // println!("Parsing expression: {:?}", lexer.peek());
    match lexer.next() {
        Some(Ok(token)) => {
            match token {
                Token::Let => parse_let(lexer),
                Token::Number(n) => Ok(Expression::Number(n)),
                Token::StringLiteral(s) => Ok(Expression::StringLiteral(s)),
                Token::Identifier(s) => { 
                    if let Some(Ok(token)) = lexer.peek() {
                        match token {
                            Token::SemiColon | Token::End | Token::In | Token::Val | Token::Fun => Ok(Expression::Identifier(s)),
                            _ => { 
                                // this is a function call
                                let mut parameters = Vec::new();
                                while let Some(Ok(token)) = lexer.peek() && 
                                    matches!(token, Token::Identifier(_) | Token::Number(_)) {
                                    parameters.push(parse_parameter(lexer)?);
                                };
                                if matches!(lexer.peek(), Some(Ok(Token::SemiColon))) { lexer.next(); } // We skip the semi colon

                                Ok(Expression::FunctionCall { name: s, parameters })
                            }
                        } 
                    } else {
                        Err(ParsingError::InvalidSyntax)
                    }                 
                }
                _ => {
                    println!("Parsing {token:?}");
                    Err(ParsingError::InvalidSyntax)
                }
            }
        }
        _ => Err(ParsingError::InvalidSyntax),
    }
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
        body.push(parse_expr(lexer)?);
    }

    assert_eq!(lexer.next(), Some(Ok(Token::End)));

    Ok(Expression::Let { declarations, body })
}

fn parse_variable(lexer: &mut PeekableLexer<'_>) -> Result<Declaration, ParsingError> {
    let name = match lexer.next() {
        Some(Ok(Token::Identifier(n))) => n,
        _ => return Err(ParsingError::InvalidSyntax),
    };

    assert_eq!(lexer.next(), Some(Ok(Token::Assignment)));

    let value = parse_expr(lexer)?;

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

    assert_eq!(lexer.next(), Some(Ok(Token::Assignment)));

    let body = parse_expr(lexer)?;

    Ok(Declaration::Function { name, parameters, body })
}

fn parse_parameter(lexer: &mut PeekableLexer<'_>) -> Result<Expression, ParsingError> {
    // println!("Parsing parameter: {:?}", lexer.peek());
    match lexer.next() {
        Some(Ok(token)) => {
            match token {
                Token::Let => parse_let(lexer),
                Token::Number(n) => Ok(Expression::Number(n)),
                Token::Identifier(s) => Ok(Expression::Identifier(s)),
                _ => {
                    println!("Parsing {token:?}");
                    Err(ParsingError::InvalidSyntax)
                }
            }
        }
        _ => Err(ParsingError::InvalidSyntax),
    }
}

#[derive(Debug)]
pub enum ParsingError {
    InvalidSyntax
}