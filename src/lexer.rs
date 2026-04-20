use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
pub enum Token {
    #[token("let")]
    Let,

    #[token("val")]
    Val,

    #[token("fun")]
    Fun,

    #[token("in", priority=20)]
    In,

    #[token("end")]
    End,

    #[regex(r"andalso|orelse|\+|-|\*|\/|<=|>=|<|>", |op| op.slice().to_owned(), priority=20)]
    Operator(String),

    #[token("(")]
    LeftParenthesis,

    #[token(")")]
    RightParenthesis,

    #[token("=")]
    EqualSign,

    #[token("if", priority=20)]
    If,

    #[token("then")]
    Then,

    #[token("else")]
    Else,

    #[token("()")]
    Unit,

    #[regex(r"[0-9]*\.?[0-9]+", |num| num.slice().parse().ok())]
    Number(f64),

    #[regex(r"true|false", |boolean| boolean.slice().parse().ok())]
    Boolean(bool),

    #[regex("\"[a-zA-Z]+\"", |lit| lit.slice().trim_matches('"').to_owned())]
    StringLiteral(String),

    #[regex(r"[a-zA-Z\_][a-zA-Z\_0-9]*", |id| id.slice().to_owned())]
    Identifier(String),

    #[token(";")]
    SemiColon,

    #[token(",")]
    Comma
}