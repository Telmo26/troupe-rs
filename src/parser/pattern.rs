pub enum Pattern {
    Identifier(String),
    Tuple(Vec<Pattern>),
    Wildcard
}