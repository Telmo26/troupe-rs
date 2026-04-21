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

    #[regex(r"raisedTo|andalso|orelse|\+|-|\*|\/|<=|>=|<|>", |op| op.slice().to_string(), priority=20)]
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

    #[regex("\"[a-zA-Z]+\"", |lit| lit.slice().trim_matches('"').to_string())]
    StringLiteral(String),

    #[regex(r"[a-zA-Z\_][a-zA-Z\_0-9]*", |id| id.slice().to_owned())]
    Identifier(String),

    #[regex(r"`\{[a-zA-Z]+\}`", |sl| sl.slice().trim_matches('`').to_string())]
    SecurityLevel(String),

    #[token("_", priority=3)]
    Wildcard,

    #[token(";")]
    SemiColon,

    #[token(",")]
    Comma
}