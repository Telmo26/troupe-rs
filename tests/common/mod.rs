#![allow(dead_code)]

use std::{fs, path::Path};

use logos::Logos;
use troupe_rs::{AST, Token, TrustMap, parse};

pub fn parse_ast(file: &str) -> datatest_stable::Result<AST> {
    let lexer = Token::lexer(file);
    let ast = parse(lexer)?;
    Ok(ast)
}

pub fn get_trustmap(path: &Path) -> datatest_stable::Result<Option<TrustMap>> {
    let trustmap_path = path.with_extension("json");

    let trustmap = if fs::exists(&trustmap_path)? {
        let json = fs::read_to_string(&trustmap_path)?;
        Some(TrustMap::from_json(&json)?)
    } else {
        None
    };
    Ok(trustmap)
}

pub fn with_settings(
    snapshot_dir: &str,
    f: impl FnOnce(),
) {
    let mut settings = insta::Settings::clone_current();

    settings.set_snapshot_path(snapshot_dir);
    settings.set_prepend_module_to_snapshot(false);
    settings.set_omit_expression(true);

    settings.bind(f);
}