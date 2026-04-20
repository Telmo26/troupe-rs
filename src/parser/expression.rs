#[derive(Debug)]
pub enum Expression {
    Let {
        declarations: Vec<Declaration>,
        body: Vec<Expression>,
    },
    FunctionCall {
        callee: Box<Expression>, 
        argument: Box<Expression>
    },
    MathOperation(String, Vec<Expression>),
    BooleanOperation(String, Vec<Expression>),
    Conditional(Box<Expression>, Box<Expression>, Option<Box<Expression>>),
    Number(f64),
    StringLiteral(String),
    Boolean(bool),
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