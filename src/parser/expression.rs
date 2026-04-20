use crate::lexer::Token;

#[derive(Debug)]
pub enum Expression {
    Let {
        declarations: Vec<Expression>,
        body: Vec<Expression>,
    },
    FunctionCall {
        callee: Box<Expression>, 
        argument: Box<Expression>
    },
    Operation(String, Vec<Expression>),
    Conditional(Box<Expression>, Box<Expression>, Option<Box<Expression>>),
    Tuple(Vec<Expression>),
    
    Unit,
    Number(f64),
    StringLiteral(String),
    Boolean(bool),
    Identifier(String),
    VariableDefinition(String, Box<Expression>),
    Lambda(Option<String>, Option<String>, Box<Expression>)
}

pub fn is_value(token: &Token) -> bool {
    match token {
        Token::Number(_)  
        | Token::Unit         
        | Token::StringLiteral(_) 
        | Token::Identifier(_)    
        | Token::Boolean(_) => true,
        _ => false
    }
}

pub fn parse_value(token: Token) -> Expression {
    match token {
        Token::Number(n) => Expression::Number(n),
        Token::Unit => Expression::Unit,
        Token::StringLiteral(s) => Expression::StringLiteral(s),
        Token::Identifier(i) => Expression::Identifier(i),
        Token::Boolean(b) => Expression::Boolean(b), 
        _ => unreachable!()
    }
}