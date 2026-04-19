#[derive(Debug)]
pub enum Expression {
    Let {
        declarations: Vec<Declaration>,
        body: Vec<Expression>,
    },
    FunctionCall {
        name: String, 
        parameters: Vec<Expression>
    },
    Operation(char, Vec<Expression>),
    Number(f64),
    StringLiteral(String),
    Identifier(String)
}

#[derive(Debug)]
pub enum Declaration {
    Variable {
        name: String,
        value: Expression,
    },
    Function {
        name: String,
        parameters: Vec<String>,
        body: Expression,
    },
}