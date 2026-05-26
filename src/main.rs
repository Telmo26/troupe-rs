use std::path::PathBuf;

use clap::Parser;
use logos::Logos;

use troupe_rs::{
    TrustMap,
    Token,
    parse,
    type_check,
    static_ifc_check,
    Interpreter
};

#[derive(Parser, Debug)]
struct Args {
    file: PathBuf,

    #[arg(long, name="trustmap.json")]
    trustmap: Option<PathBuf>,

    #[arg(long, name = "type-checker", default_value = "false")]
    /// Enables the optional type checker. Note that valid type-checked
    /// programs are a strict subset of valid Troupe programs
    r#type_checker: bool
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

    dbg!(&ast);

    if args.type_checker {
        type_check(&ast).expect("Type check failed");
    }
    
    static_ifc_check(&ast, false, trustmap).expect("Static analysis failed.");

    let mut interpreter = Interpreter::default();
    interpreter.run(ast);

    Ok(())
}
