//! Compiler frontend checks for editor tooling; never invokes a native compiler.
use dev_syntax::{ast, parser};
mod callbacks;
pub mod codegen;
mod collections;
pub mod program;
mod tasks;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub fn check_sources(
    entry: &Path,
    directories: &HashMap<String, PathBuf>,
    sources: &HashMap<PathBuf, String>,
) -> Vec<String> {
    match program::Program::load_with_sources(entry, directories, sources) {
        Ok(program) => codegen::diagnostics(&program),
        Err(error) => vec![error],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn errors(source: &str) -> Vec<String> {
        let entry = std::env::temp_dir().join("devlang-virtual-editor-test.dev");
        check_sources(
            &entry,
            &HashMap::new(),
            &HashMap::from([(entry.clone(), source.into())]),
        )
    }
    #[test]
    fn collects_independent_statement_errors() {
        let result = errors(
            "fn main() {\nlet age i64 = \"wrong\"\nprint(missing)\nif 42 { print(1) }\n}\nmain()",
        );
        assert!(result.len() >= 3, "{result:?}");
        assert!(result.iter().any(|e| e.contains("missing")), "{result:?}");
    }
    #[test]
    fn checks_calls_returns_and_redeclarations() {
        for source in [
            "fn add(a i64, b i64) i64 { return a + b }\nadd(1)",
            "fn add(a i64) i64 { return a }\nadd(\"bad\")",
            "fn value() i64 { return \"bad\" }",
            "fn value() i64 { print(1) }",
            "fn main() { let x = 1; let x = 2 }",
            "fn main(x i64, x i64) {}",
            "fn main() { let x u8 = 999 }",
            "fn main() { break }",
            "fn main() { let x = 1 + true }",
        ] {
            assert!(
                !errors(source).is_empty(),
                "accepted invalid source: {source}"
            );
        }
    }
    #[test]
    fn valid_code_has_no_diagnostics() {
        for source in [
            "fn add(a i64, b i64) i64 { return a + b }\nprint(add(1, 2))",
            "fn main() { let age u8 = 18; print(age) }\nmain()",
            "struct User { age i64 }\nlet user = User(18)\nprint(user.age)",
            "fn identity<T>(x T) T { return x }\nprint(identity<i64>(42))",
        ] {
            assert!(errors(source).is_empty(), "{source}: {:?}", errors(source));
        }
    }
    #[test]
    fn checks_unsaved_imports_without_writing_files() {
        let root = std::env::temp_dir().join("devlang-virtual-editor-modules");
        let entry = root.join("main.dev");
        let helper = root.join("helper.dev");
        let mut sources = HashMap::from([
            (
                entry.clone(),
                "use \"helper\"\nprint(helper.value())".into(),
            ),
            (helper.clone(), "fn value() str { return \"hello\" }".into()),
        ]);
        assert!(check_sources(&entry, &HashMap::new(), &sources).is_empty());
        sources.insert(
            entry.clone(),
            "use \"helper\"\nlet result i64 = helper.value()\nprint(result)".into(),
        );
        assert!(!check_sources(&entry, &HashMap::new(), &sources).is_empty());
        sources.insert(helper, "fn value() i64 { return 42 }".into());
        assert!(check_sources(&entry, &HashMap::new(), &sources).is_empty());
    }
}
