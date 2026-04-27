use std::ops::Range;

use logos::SpannedIter;

use crate::{lexer::Token, parser::ast::MatchClause};

mod ast;
use ast::{AST, Pattern};

mod utils;
use utils::{is_value, parse_value};

mod error;
use error::ParsingError;

mod position_calculator;
use position_calculator::PositionCalculator;

pub fn parse(lexer: logos::Lexer<'_, Token>) -> Result<AST, ParsingError> {
    let position_calculator = PositionCalculator::new(lexer.source());
    let mut parser = Parser { 
        lexer: lexer.spanned(),
        peeked: None,
        position_calculator,
    };
    parser.parse_expr(0)
}

struct Parser<'a> {
    lexer: SpannedIter<'a, Token>,
    peeked: Option<Result<(Token, Range<usize>), ParsingError>>,
    position_calculator: PositionCalculator,
}

// These are utility wrapper functions around the lexer;
impl<'a> Parser<'a> {
    fn peek(&mut self) -> Result<&(Token, Range<usize>), ParsingError> {
        let iter = &mut self.lexer;
        let val = self.peeked.get_or_insert_with(||
            iter.next().map_or_else(|| Err(ParsingError::EOF), |(token, span)| {
                match token {
                    Ok(token) => Ok((token, span)),
                    Err(_) => Err(ParsingError::InvalidToken { 
                        token: iter.slice().to_string(), 
                        position: self.position_calculator.compute(span.start)
                    }),
                }
            })
        ).as_ref();
        
        return val.map_err(|err| err.clone());
    }

    fn next(&mut self) -> Result<(Token, Range<usize>), ParsingError> {
        let result = match self.peeked.take() {
            Some(v) => v,
            None => match self.lexer.next() {
                Some((Ok(token), pos)) => Ok((token, pos)),
                Some((Err(_), span)) => Err(ParsingError::InvalidToken { 
                    token: self.lexer.slice().to_string(), 
                    position: self.position_calculator.compute(span.start) 
                }),
                None => Err(ParsingError::EOF)
            },
        };
        
        result
    }

    fn expect(&mut self, expected: Token) -> Result<(), ParsingError> {
        let (token, span) = self.next()?;
        if token == expected {
            Ok(())
        } else {
            Err(ParsingError::UnexpectedToken { 
                message: "An expected token is missing",
                expected: vec![expected.to_string()], 
                found: token.to_string(), 
                position: self.position_calculator.compute(span.start),
            })
        }
    }

    fn next_if(&mut self, expected: Token) -> Result<bool, ParsingError> {
        if self.peek()?.0 == expected {
            self.next()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

// These are the real deal
impl<'a> Parser<'a> {
    fn parse_expr(&mut self, min_bp: u8) -> Result<AST, ParsingError> {
        println!("Parsing expression: {:?}", self.peek());
        let (token, span) = self.next()?;
        let mut lhs = match token {
            Token::Unit => return Ok(AST::Unit),
            t if is_value(&t) => parse_value(t),
            Token::Import => {
                self.next()?; // We ignore the imported module
                return self.parse_expr(min_bp)
            }
            Token::Wildcard => return Ok(AST::Wildcard),
            Token::Let => self.parse_let()?,
            Token::Val => {
                let (name, value) = self.parse_variable()?;
                self.parse_declaration(name, value)?
            },
            Token::Fun => {
                let (name, value) = self.parse_function(true)?;
                self.parse_declaration(name, value)?
            },
            Token::Fn => {
                let (_, value) = self.parse_function(false)?;
                value
            }
            Token::Case             => self.parse_case()?,
            Token::If               => self.parse_conditional()?,
            Token::LeftParenthesis  => self.parse_parenthesis()?,
            Token::LeftBracket      => self.parse_list()?,
            Token::Handler          => self.parse_handler()?,
            Token::Operator(op) if op == "-" => {
                let rhs = self.parse_expr(0)?;
                AST::Operation(op, vec![rhs])
            }
            _ => return Err(ParsingError::UnexpectedToken { 
                message: "An invalid token started an expression",
                expected: vec![],
                found: token.to_string(),
                position: self.position_calculator.compute(span.start),
            })
        };

        loop {
            let res = self.peek();
            if res.is_err() {
                break;
            }
            match res.unwrap() {
                (Token::SemiColon, _) => {
                    self.next()?;
                    lhs = self.parse_declaration(Pattern::Empty, lhs)?;
                    break;
                }

                // For a function unit must be its only parameter
                (Token::Unit, _) => {
                    self.next()?;
                    lhs = AST::FunctionCall { 
                        callee: Box::new(lhs), 
                        argument: Box::new(AST::Unit) 
                    };
                    break;
                }

                // This is for tuple or list function parameters
                (Token::LeftParenthesis, _) | (Token::LeftBracket, _) => {
                    let argument = Box::new(self.parse_expr(FUNCTION_CALL_BINDING_POWER)?);
                    lhs = AST::FunctionCall { callee: Box::new(lhs), argument };
                    continue;
                }

                (Token::Operator(op), _) => {
                    let op = op.clone();
                    if let Some((l_bp, r_bp)) = infix_binding_power(&op) {
                        if l_bp < min_bp {
                            break;
                        }
                        self.next()?;
                        let rhs = self.parse_expr(r_bp)?;
                        lhs = AST::Operation(op, vec![lhs, rhs]);
                        continue;
                    }
                },

                (Token::EqualSign, _) => {
                    if let Some((l_bp, r_bp)) = infix_binding_power("=") {
                        if l_bp < min_bp {
                            break;
                        }
                        self.next()?;
                        let rhs = self.parse_expr(r_bp)?;
                        lhs = AST::Operation("=".to_owned(), vec![lhs, rhs]);
                        continue;
                    }
                }

                (token, _) if is_value(token) => {
                    let argument = parse_value(self.next().unwrap().0);
                    lhs = AST::FunctionCall { callee: Box::new(lhs), argument: Box::new(argument) };
                    continue;
                },
                
                _ => break,
            };
        }

        Ok(lhs)
    }

    fn parse_let(&mut self) -> Result<AST, ParsingError> {
        let (name, value) = match self.next()? {
            (Token::Val, _) => self.parse_variable()?,
            (Token::Fun, _) => self.parse_function(true)?,
            (token, span) => return Err(ParsingError::UnexpectedToken { 
                message: "Invalid binding keyword",
                expected: vec![Token::Val.to_string(), Token::Fun.to_string()], 
                found: token.to_string(), 
                position: self.position_calculator.compute(span.start)
            })
        };

        let declaration = self.parse_declaration(name, value);

        self.next_if(Token::End)?;

        declaration
    }

    fn parse_declaration(&mut self, name: Pattern, value: AST) -> Result<AST, ParsingError> {
        self.next_if(Token::In)?;

        let body = Box::new(self.parse_expr(0)?);

        Ok(AST::Let { 
            name, 
            value: Box::new(value), 
            body,
        })
    }

    fn parse_variable(&mut self) -> Result<(Pattern, AST), ParsingError> {
        println!("Parsing variable: {:?}", self.peek());

        match self.peek()? {
            // This is the normal assignment or pattern-matching
            (Token::Identifier(_), _) | (Token::LeftParenthesis, _) =>{
                let pattern = self.parse_pattern()?;
                
                self.expect(Token::EqualSign)?;

                let value = self.parse_expr(0)?;

                Ok((pattern, value))
            },

            (Token::Wildcard, _) => {
                self.next()?;
                self.expect(Token::EqualSign)?;

                let value = self.parse_expr(0)?;

                Ok((Pattern::Empty, value))
            }
            _ => {
                let (token, span) = self.next()?;
                Err(ParsingError::UnexpectedToken { 
                    message: "Invalid variable name or deconstruction pattern",
                    expected: vec!["Identifier".to_string(), Token::Wildcard.to_string()], 
                    found: token.to_string(), 
                    position: self.position_calculator.compute(span.start)
                })
            },
        }    
    }

    fn parse_function(&mut self, named: bool) -> Result<(Pattern, AST), ParsingError> {
        println!("Parsing function: {:?}", self.peek());
        // We get the function's name if it is named
        let name = if named {
            match self.next()? {
                (Token::Identifier(n), _) => Pattern::Single(Box::new(AST::Identifier(n))),
                (Token::Wildcard, _) => Pattern::Empty,
                (token, span) => return Err(ParsingError::UnexpectedToken { 
                    message: "Invalid function name declaration",
                    expected: vec!["<IDENTIFIER>".to_string(), Token::Wildcard.to_string()], 
                    found: token.to_string(), 
                    position: self.position_calculator.compute(span.start)
                }),
            }
        } else { Pattern::Empty };

        let variable = match self.peek()? {
            (Token::Identifier(_), _) => {
                let Ok((Token::Identifier(p), _)) = self.next() else { unreachable!() };
                Some(p)
            }
            (Token::Unit, _) => {
                self.next()?;
                None
            }
            _ => {
                let (token, span) = self.next()?;
                return Err(ParsingError::UnexpectedToken { 
                    message: "Invalid function parameter declaration",
                    expected: vec!["<IDENTIFIER>".to_string(), Token::Unit.to_string()], 
                    found: token.to_string(), 
                    position: self.position_calculator.compute(span.start)
                })
            }
        };

        match self.peek()? {
            // We recursively descend if the function has more variables
            (Token::Identifier(_), _) => {
                let (_, body) = self.parse_function(false)?;
                Ok((name, AST::Lambda(variable, Box::new(body))))
            }

            // Otherwise we simply compute the body
            (Token::EqualSign, _) | (Token::Arrow, _) => {
                self.next()?;
                let body = Box::new(self.parse_expr(0)?);
                Ok((name, AST::Lambda(variable, body)))
            }

            _ => {
                let (token, span) = self.next()?;
                return Err(ParsingError::UnexpectedToken { 
                    message: "Invalid multi-parameter function declaration",
                    expected: vec!["<IDENTIFIER>".to_string(), Token::EqualSign.to_string()], 
                    found: token.to_string(), 
                    position: self.position_calculator.compute(span.start)
                })
            }
        }
    }

    fn parse_conditional(&mut self) -> Result<AST, ParsingError> {
        let condition = Box::new(self.parse_expr(0)?);

        self.expect(Token::Then)?;

        let first_path = Box::new(self.parse_expr(0)?);

        let mut second_path = None;
        if self.next_if(Token::Else)? {
            second_path = Some(Box::new(self.parse_expr(0)?));
        };

        Ok(AST::Conditional(condition, first_path, second_path))
    }

    fn parse_parenthesis(&mut self) -> Result<AST, ParsingError> {
        let lhs = self.parse_expr(0)?;

        if self.next_if(Token::RightParenthesis)? {
            return Ok(lhs)
        }

        let mut expressions = vec![lhs];
        while self.next_if(Token::Comma)? {
            expressions.push(self.parse_expr(0)?);
        }

        self.expect(Token::RightParenthesis)?;

        Ok(AST::Tuple(expressions))
    }

    fn parse_case(&mut self) -> Result<AST, ParsingError> {
        let expression = Box::new(self.parse_expr(0)?);

        self.expect(Token::Of)?;

        let mut clauses: Vec<MatchClause> = vec![];
        loop {
            let pattern = self.parse_pattern()?;

            let guard = if self.next_if(Token::When)? {
                Some(self.parse_expr(0)?)
            } else { None };

            self.expect(Token::Arrow)?;

            let body = self.parse_expr(0)?;

            clauses.push(MatchClause {
                pattern,
                guard,
                body
            });

            if !self.next_if(Token::Disjunction)? {
                break;
            }
        }

        Ok(AST::Case(
            expression, 
            clauses
        ))
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ParsingError> {
        println!("Parsing pattern : {:?}", self.peek());
        match self.peek()? {
            (Token::LeftParenthesis, _) => {
                self.next()?;
                let mut patterns = vec![self.parse_pattern()?];
                while self.next_if(Token::Comma)? {
                    patterns.push(self.parse_pattern()?);
                }

                self.expect(Token::RightParenthesis)?;

                Ok(Pattern::Tuple(patterns))
            }
            (t, _) if is_value(t) => {
                let value = self.next().unwrap().0;
                Ok(Pattern::Single(Box::new(parse_value(value))))
            }
            (Token::Wildcard, _) => {
                self.next()?;
                Ok(Pattern::Empty)
            }
            _ => {
                let (token, span) = self.next()?;
                Err(ParsingError::UnexpectedToken { 
                    message: "Invalid pattern",
                    expected: vec!["<VALUE>".to_string(), "<IDENTIFIER>".to_string(),Token::Wildcard.to_string(), Token::LeftParenthesis.to_string()], 
                    found: token.to_string(), 
                    position: self.position_calculator.compute(span.start)
                })
            }
        }
    }

    fn parse_list(&mut self) -> Result<AST, ParsingError> {
        let mut expressions = vec![];

        if self.peek()?.0 != Token::RightBracket {
            loop {
                expressions.push(self.parse_expr(0)?);          
                if !self.next_if(Token::Comma)? { 
                    break; 
                }
            }
        }

        self.expect(Token::RightBracket)?;

        Ok(AST::List(expressions))
    }

    fn parse_handler(&mut self) -> Result<AST, ParsingError> {
        let pattern = self.parse_pattern()?;

        let guard = if self.next_if(Token::When)? {
            Some(self.parse_expr(0)?)
        } else { None };

        self.expect(Token::Arrow)?;

        let body = self.parse_expr(0)?;

        Ok(AST::Case(
            Box::new(AST::Identifier("_handlerInput".to_string())), 
            vec![MatchClause {
                pattern,
                guard,
                body
            }]
        ))
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
        "raisedTo" | "::" => Some((1, 2)),
        _ => None
    }
}