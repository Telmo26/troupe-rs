// #![allow(dead_code)]

#[derive(Debug)]
pub enum AST {
    Let {
        name: Pattern,
        value: Box<AST>,
        body: Box<AST>,
    },
    FunctionCall {
        callee: Box<AST>, 
        argument: Box<AST>
    },
    Operation(String, Vec<AST>),
    Conditional(Box<AST>, Box<AST>, Option<Box<AST>>),
    Tuple(Vec<AST>),
    
    Unit,
    Wildcard,
    Number(f64),
    StringLiteral(String),
    Boolean(bool),
    Identifier(String),
    Lambda(Option<String>, Box<AST>),
    SecurityLevel(String),
}

#[derive(Debug)]
pub enum Pattern {
    Identifier(String),
    Tuple(Vec<AST>),
    Empty
}