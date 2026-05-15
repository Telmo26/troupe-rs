use std::path::PathBuf;

use clap::Parser;
use logos::Logos;

mod lexer;
mod static_ifc;
mod type_checker;

use lexer::Token;

use crate::{parser::parse, static_ifc::static_ifc_check, type_checker::type_check};

mod parser;

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
    static_ifc_check(&ast, false).expect("Static analysis failed.");

    Ok(())
}
