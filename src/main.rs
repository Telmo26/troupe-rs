use std::path::PathBuf;

use clap::Parser;
use logos::Logos;

mod lexer;
mod parser;
mod type_checker;
mod interpreter;

use crate::{
    interpreter::Interpreter, lexer::Token, parser::parse, type_checker::type_check
};

#[derive(Parser, Debug)]
struct Args {
    file: PathBuf,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let file_text = std::fs::read_to_string(args.file)?;

    let lexer = Token::lexer(&file_text);
    let ast = parse(lexer).unwrap();

    type_check(&ast).expect("Type check failed");

    let mut interpreter = Interpreter::default();
    interpreter.run(ast);

    Ok(())
}
