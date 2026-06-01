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

impl AST {
    pub fn contains_identifier(&self, id: &str) -> bool {
        use AST::*;
        match self {
            Identifier(ident) if ident == id => true,
            Unit | Number(_) | StringLiteral(_) | Boolean(_) | SecurityLevel(_) | Unreachable | Identifier(_) => false,
            
            Let { value, body, ..} => value.contains_identifier(id) | body.contains_identifier(id),
            FunctionCall { callee, argument } => callee.contains_identifier(id) || argument.contains_identifier(id),
            Operation(_, values) | Tuple(values) | List(values) => values.iter().any(|v| v.contains_identifier(id)),
            Conditional(cond, b1, b2) => {
                let b2 = match b2 {
                    None => false,
                    Some(branch) => branch.contains_identifier(id)
                };
                cond.contains_identifier(id) || b1.contains_identifier(id) || b2
            },
            Case(value, clauses) => {
                let clauses_bool = clauses.iter().any(|c| {
                    let guard = match c.guard.as_ref() {
                        None => false,
                        Some(g) => g.contains_identifier(id)
                    };
                    c.body.contains_identifier(id) || guard
                });
                value.contains_identifier(id) || clauses_bool
            },
            Lambda(_, body) => body.contains_identifier(id)
        }
    }
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
