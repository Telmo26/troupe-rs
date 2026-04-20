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

    #[token("in")]
    In,

    #[token("end")]
    End,

    #[regex(r"andalso|orelse|\+|-|\*|\\|\(|\)|<=|>=|<|>", |op| op.slice().to_owned(), priority=20)]
    Operator(String),

    #[token("=")]
    EqualSign,

    #[token("if")]
    If,

    #[token("then")]
    Then,

    #[token("else")]
    Else,

    #[regex(r"[0-9]*\.?[0-9]+", |num| num.slice().parse().ok())]
    Number(f64),

    #[regex(r"true|false", |boolean| boolean.slice().parse().ok())]
    Boolean(bool),

    #[regex("\"[a-zA-Z]+\"", |lit| lit.slice().trim_matches('"').to_owned())]
    StringLiteral(String),

    #[regex(r"[a-zA-Z\_]+", |id| id.slice().to_owned())]
    Identifier(String),

    #[token(";")]
    SemiColon,
}