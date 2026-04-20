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
    
    Unit,
    Number(f64),
    StringLiteral(String),
    Boolean(bool),
    Identifier(String),
    VariableDefinition(String, Box<Expression>),
    Lambda(Option<String>, Option<String>, Box<Expression>)
}