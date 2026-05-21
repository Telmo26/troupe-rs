#[derive(Debug, Clone, PartialEq)]
pub enum AST {
    Let {
        name: Pattern,
        value: Box<AST>,
        body: Box<AST>,
        rec: bool,
    },
    FunctionCall {
        callee: Box<AST>,
        argument: Box<AST>,
    },
    Operation(String, Vec<AST>),
    Case(Box<AST>, Vec<MatchClause>),
    Conditional(Box<AST>, Box<AST>, Option<Box<AST>>),
    Tuple(Vec<AST>),
    List(Vec<AST>),

    Unit,
    Number(f64),
    StringLiteral(String),
    Boolean(bool),
    Identifier(String),
    Lambda(Option<String>, Box<AST>),
    SecurityLevel(Vec<String>),

    Unreachable
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Variable(String),
    Value(Box<AST>),
    Tuple(Vec<Pattern>),
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchClause {
    pub pattern: Pattern,
    pub guard: Option<AST>,
    pub body: AST,
}
