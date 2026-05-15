use std::path::PathBuf;

use clap::Parser;
use logos::Logos;

mod lexer;
mod parser;
mod type_checker;
mod interpreter;
mod trustmap;

use crate::{
    interpreter::Interpreter, lexer::Token, parser::parse, trustmap::TrustMap, type_checker::type_check
};

#[derive(Parser, Debug)]
struct Args {
    file: PathBuf,

    #[arg(long, name="trustmap.json")]
    trustmap: Option<PathBuf>
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let file_text = std::fs::read_to_string(args.file)?;
    let trustmap = if let Some(tm_path) = args.trustmap {
        let json = std::fs::read_to_string(tm_path)?;
        Some(TrustMap::from_json(&json)?)
    } else {
        None
    };

    if let Some(tm) = trustmap.as_ref() {
        dbg!(tm);
    }

    let lexer = Token::lexer(&file_text);
    let ast = parse(lexer).unwrap();

    // type_check(&ast).expect("Type check failed");

    let mut interpreter = Interpreter::default();
    interpreter.run(ast);

    Ok(())
}
