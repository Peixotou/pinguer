//! Snapshot tests: the syntax trees of the programs in `examples/`.
//!
//! The expected tree of `examples/<name>.pgr` is in
//! `tests/snapshots/<name>.ast`, in the format of `pinguer debug ast`. After
//! an intended change, run the tests with `PINGUER_UPDATE_SNAPSHOTS=1` to
//! rewrite the snapshots, and review the difference before committing it.

use std::fs;
use std::path::{Path, PathBuf};

use pinguer_diagnostics::Diagnostics;
use pinguer_source::SourceMap;

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn snapshot_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(format!("{name}.ast"))
}

#[test]
fn examples_match_their_snapshots() {
    let mut names: Vec<String> = fs::read_dir(examples_dir())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "pgr"))
        .map(|path| path.file_stem().unwrap().to_str().unwrap().to_owned())
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no examples found");
    let update = std::env::var_os("PINGUER_UPDATE_SNAPSHOTS").is_some();
    for name in names {
        let mut map = SourceMap::new();
        let file = map
            .load(&examples_dir().join(format!("{name}.pgr")))
            .unwrap();
        let source = map.file(file);
        let mut diagnostics = Diagnostics::new();
        let tokens = pinguer_lexer::tokenize(file, source.text(), &mut diagnostics);
        let tree = pinguer_parser::parse(file, source.text(), tokens, &mut diagnostics);
        let messages: Vec<&str> = diagnostics.iter().map(|d| d.message.as_str()).collect();
        assert!(messages.is_empty(), "{name}: {messages:?}");
        let rendered = pinguer_ast::render_to_string(&tree, source);
        let path = snapshot_path(&name);
        if update {
            fs::write(&path, &rendered).unwrap();
            continue;
        }
        let expected = fs::read_to_string(&path).unwrap_or_else(|_| {
            panic!(
                "{name}: no snapshot at {}; run with PINGUER_UPDATE_SNAPSHOTS=1",
                path.display()
            )
        });
        assert_eq!(rendered, expected, "{name}");
    }
}
