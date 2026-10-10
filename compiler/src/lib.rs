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
    fn core_modules_have_valid_editor_signatures() {
        for module in [
            "net", "path", "os", "stream", "url", "module", "process", "events", "buffer", "dgram",
            "math", "random", "datetime", "test", "log", "strings",
            "regex", "encoding", "crypto", "compression", "archive", "uuid",
            "dns", "cli",
            "result", "timers", "child_process",
            "sqlite", "csv", "toml", "yaml", "tls", "websocket",
        ] {
            let source = format!("use \"std/{module}\"\nfn main() {{}}\nmain()");
            assert!(
                errors(&source).is_empty(),
                "{module}: {:?}",
                errors(&source)
            );
        }
        let source = "use \"std/dns\"\nuse \"std/cli\"\nfn main() {\nlet options=Vec<cli.Option>()\noptions.push(cli.Option(\"port\",\"p\",true))\nlet parsed=cli.parse(cli.args(),options)\nif parsed.values.contains(\"port\") { print(parsed.values.get(\"port\")) }\nlet ip=dns.lookupOne(\"localhost\",4)\nlet family i64=ip.family\nprint(family)\n}\nmain()";
        assert!(errors(source).is_empty(), "{:?}", errors(source));
        for source in [
            "use \"std/path\"\npath.join(42, \"x\")",
            "use \"std/net\"\nnet.connect(80, 42, 1000)",
            "use \"std/buffer\"\nbuffer.alloc(\"4\", 0 as u8)",
            "use \"std/events\"\nlet e = events.createEmitter()\ne.on(\"x\", fn(value i64) {})",
            "use \"std/math\"\nmath.sqrt(\"wrong\")",
            "use \"std/random\"\nrandom.int(1, \"wrong\")",
            "use \"std/strings\"\nstrings.trim(1)",
            "use \"std/datetime\"\ndatetime.parts(0, \"UTC\")",
            "use \"std/test\"\ntest.case(\"wrong\", fn(n i64) {})",
            "use \"std/log\"\nlog.info(1)",
            "use \"std/regex\"\nregex.compile(1, \"\")",
            "use \"std/encoding\"\nencoding.decode(\"bytes\", \"utf8\")",
            "use \"std/crypto\"\ncrypto.secureBytes(\"32\")",
            "use \"std/compression\"\ncompression.gzip(\"bytes\", 6)",
            "use \"std/archive\"\narchive.writeZIP(Vec<i64>())",
            "use \"std/uuid\"\nuuid.parse(42)",
            "use \"std/dns\"\ndns.lookup(42, 4)",
            "use \"std/dns\"\ndns.lookup(\"localhost\", \"IPv4\")",
            "use \"std/cli\"\ncli.parse(Vec<str>(), Vec<i64>())",
            "use \"std/result\"\nresult.attempt(42)",
            "use \"std/timers\"\ntimers.setTimeout(fn() {}, 1)",
            "use \"std/child_process\"\nchild_process.spawn(42, Vec<str>(), child_process.options())",
            "use \"std/sqlite\"\nsqlite.open(42)",
            "use \"std/csv\"\ncsv.parse(42,\",\",true)",
            "use \"std/toml\"\ntoml.parse(42)",
            "use \"std/yaml\"\nyaml.valid(42)",
            "use \"std/tls\"\ntls.createServer(\"cert\",\"key\",fn(s i64) {})",
            "use \"std/websocket\"\nwebsocket.connect(42,1000,\"\")",
        ] {
            assert!(!errors(source).is_empty(), "accepted: {source}");
        }
    }
    #[test]
    fn infers_generic_function_parameters_and_result_payloads() {
        let source = "use \"std/result\"\nfn apply<T>(body fn() T) T { return body() }\nfn main() {\nlet answer i64=apply(fn() i64 { return 42 })\nlet outcome=result.attempt(fn() str { return \"ok\" })\nlet text str=result.unwrapOr(outcome,\"fallback\")\nmatch outcome { Ok(value) => { print(value) } Err(message) => { print(message) } }\nprint(answer)\nprint(text)\n}\nmain()";
        assert!(errors(source).is_empty(), "{:?}", errors(source));
        assert!(!errors("fn select<T>(a fn() T, b fn() T) T { return a() }\nselect(fn() i64 { return 1 },fn() str { return \"x\" })").is_empty());
    }
    #[test]
    fn runtime_io_signatures_are_checked_without_execution() {
        for source in [
            "use \"std/fs/promises\"\nfn main() { let text = await(promises.readFile(\"missing\", \"utf8\")); print(text) }\nmain()",
            "use \"std/http\"\nfn main() { let server = http.createServer(fn(req http.IncomingMessage, res http.ServerResponse) { res.end(req.url) }); server.listen(3000) }\nmain()",
        ] {
            assert!(errors(source).is_empty(), "{:?}", errors(source));
        }
        let valid = "use \"std/fs\"\nuse \"std/http\"\nfn main() {\nlet text str = fs.read_text(\"missing-file\")\nlet response = http.get(\"https://invalid.example\")\nlet status i64 = response.status\nlet ok bool = response.ok\nprint(text)\nprint(status)\nprint(ok)\n}\nmain()";
        assert!(errors(valid).is_empty(), "{:?}", errors(valid));
        for source in [
            "use \"std/fs\"\nfs.readFileSync(\"missing\", 42)",
            "use \"std/fs/promises\"\nlet text str = promises.readFile(\"missing\", \"utf8\")",
            "use \"std/http\"\nhttp.createServer(fn(req i64, res i64) {})",
            "use \"std/fs\"\nfs.read_text(12)",
            "use \"std/http\"\nhttp.get(12)",
            "use \"std/http\"\nlet r = http.get(\"https://invalid.example\")\nlet status str = r.status",
        ] {
            assert!(!errors(source).is_empty(), "accepted: {source}");
        }
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
