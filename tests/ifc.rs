use std::path::Path;

use troupe_rs::static_ifc_check;

mod common;
use common::{
    parse_ast,
    get_trustmap,
    with_settings
};

fn ifc_test(path: &Path, file: String, snapshot_path: &str) -> datatest_stable::Result<()> {
    let ast = parse_ast(&file)?;
    let trustmap = get_trustmap(path)?;

    let result = static_ifc_check(&ast, false, trustmap);

    with_settings(snapshot_path, || {
        insta::assert_debug_snapshot!(
            path.file_stem().unwrap().to_str().unwrap(),
            result
        )}
    );
    
    Ok(())
}

fn ifc_positive_test(path: &Path, file: String) -> datatest_stable::Result<()> {
    ifc_test(path, file, "snapshots/ifc/positive")
}

fn ifc_negative_test(path: &Path, file: String) -> datatest_stable::Result<()> {
    ifc_test(path, file, "snapshots/ifc/negative")
}

datatest_stable::harness! {
    { test = ifc_positive_test, root = "tests/ifc/positive", pattern = r".*.trp" },
    { test = ifc_negative_test, root = "tests/ifc/negative", pattern = r".*.trp" }
}