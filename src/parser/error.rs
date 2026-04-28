use std::{error::Error, fmt::Display};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ParsingError {
    EOF,
    UnexpectedToken {
        message: &'static str,
        expected: Vec<String>,
        found: String,
        position: (usize, usize),
    },
    InvalidToken {
        token: String,
        position: (usize, usize),
    },
}

impl Error for ParsingError {}

impl Display for ParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
