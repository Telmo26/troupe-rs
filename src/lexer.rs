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

    #[token("=")]
    Assignment,

    #[regex(r"[+-]?([0-9]*\.?[0-9]+)", |num| num.slice().parse().ok())]
    Number(f64),

    #[regex(r"/[+\-\*\/\(\)]", |op| op.slice().chars().next().unwrap())]
    Operator(char),

    #[regex("\"[a-zA-Z]+\"", |lit| lit.slice().trim_matches('"').to_owned())]
    StringLiteral(String),

    #[regex(r"[a-zA-Z]+", |id| id.slice().to_owned())]
    Identifier(String),

    #[token(";")]
    SemiColon,
}