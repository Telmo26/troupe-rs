use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+")]
#[logos(skip r"\(\*[a-zA-Z0-9 ]*\*\)")]
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

    #[regex(r"::|raisedTo|andalso|orelse|\+|-|\*|\/|<=|>=|<|>", |op| op.slice().to_string(), priority=20)]
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

    #[regex(r"[a-zA-Z\_]+[0-9]*", |id| id.slice().to_string())]
    Identifier(String),

    #[regex(r"`\{[a-zA-Z]+\}`", |sl| sl.slice().trim_matches('`').to_string())]
    SecurityLevel(String),

    #[token("case")]
    Case,

    #[token("of")]
    Of,

    #[token("=>")]
    Arrow,

    #[token("|")]
    Disjunction,

    #[token("[")]
    LeftBracket,

    #[token("]")]
    RightBracket,

    #[token("hn")]
    Handler,

    #[token("when")]
    When,

    #[token("fn")]
    Fn,

    #[token("_", priority=3)]
    Wildcard,

    #[token(";")]
    SemiColon,

    #[token(",")]
    Comma,

    #[token("import")]
    Import,
}