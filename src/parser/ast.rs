// #![allow(dead_code)]

#[derive(Debug, Clone)]
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
    Match(Pattern, Box<AST>),
    Conditional(Box<AST>, Box<AST>, Option<Box<AST>>),
    Tuple(Vec<AST>),
    List(Vec<AST>),
    
    Unit,
    Wildcard,
    Number(f64),
    StringLiteral(String),
    Boolean(bool),
    Identifier(String),
    Lambda(Option<String>, Box<AST>),
    SecurityLevel(String),
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Single(Box<AST>),
    Tuple(Vec<AST>),
    Empty
}