use std::path::Path;

use troupe_rs::Interpreter;

mod common;
use common::{
    parse_ast,
    with_settings
};

fn runtime_test(path: &Path, file: String) -> datatest_stable::Result<()> {
    let ast = parse_ast(&file)?;

    let mut interpreter = Interpreter::default();
    let result = interpreter.eval(ast);

    with_settings("snapshots/runtime", || {
        insta::assert_debug_snapshot!(
            path.file_stem().unwrap().to_str().unwrap(),
            result
        )}
    );
    
    Ok(())
}

datatest_stable::harness! {
    { test = runtime_test, root = "tests/runtime", pattern = r".*.trp" }
}