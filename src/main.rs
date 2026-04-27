use std::path::PathBuf;

use clap::Parser;
use logos::Logos;

mod lexer;
use lexer::Token;

use crate::parser::parse;

mod parser;

#[derive(Parser, Debug)]
struct Args {
    file: PathBuf
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let file_text = std::fs::read_to_string(args.file)?;

    let lexer = Token::lexer(&file_text);
    let ast = parse(lexer);
    
    match ast {
        Ok(tree) => println!("{tree:#?}"),
        Err(e) => eprintln!("{e}"),
    };

    Ok(())
}
