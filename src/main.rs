use std::path::PathBuf;

use clap::Parser;
use logos::Logos;

mod lexer;
mod parser;
mod type_checker;
mod static_ifc;
mod interpreter;
mod trustmap;

use crate::{
    lexer::Token, 
    parser::parse, 
    type_checker::type_check,
    static_ifc::static_ifc_check,
    interpreter::Interpreter, 
    trustmap::TrustMap, 
};

#[derive(Parser, Debug)]
struct Args {
    file: PathBuf,

    #[arg(long, name="trustmap.json")]
    trustmap: Option<PathBuf>
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    // dbg!(&ast);

    // type_check(&ast).expect("Type check failed");
    // static_ifc_check(&ast, trustmap, false).expect("Static analysis failed.");

    let interpreter = Interpreter::new();
    interpreter.run(ast).await;

    Ok(())
}
