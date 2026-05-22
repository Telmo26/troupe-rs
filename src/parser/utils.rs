use crate::{lexer::Token, parser::ast::AST};

pub fn is_value(token: &Token) -> bool {
    match token {
        Token::Number(_)  
        | Token::SecurityLevel(_)
        | Token::Unit         
        | Token::StringLiteral(_) 
        | Token::Identifier(_)    
        | Token::Boolean(_) => true,
        _ => false
    }
}

pub fn parse_value(token: Token) -> AST {
    match token {
        Token::Number(n) => AST::Number(n),
        Token::Unit => AST::Unit,
        Token::StringLiteral(s) => AST::StringLiteral(s),
        Token::Identifier(i) => AST::Identifier(i),
        Token::Boolean(b) => AST::Boolean(b), 
        Token::SecurityLevel(s) => {
            let levels = s[1..s.len()-1].split(",")
                .map(|l| l.trim().to_string())
                .filter(|s| !s.is_empty()) // If we don't do that the level {} will be parsed as "" instead of lack of level
                .collect();
            AST::SecurityLevel(levels)
        }
        _ => unreachable!()
    }
}