"""Verify real LSP semantic diagnostics, unsaved text and imported-file refresh."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile


def run(binary, messages):
    wire = b""
    for message in messages:
        body = json.dumps(dict(jsonrpc="2.0", **message), ensure_ascii=False).encode()
        wire += f"Content-Length: {len(body)}\r\n\r\n".encode() + body
    result = subprocess.run([str(binary), "lsp", "--stdio"], input=wire,
                            capture_output=True, timeout=30)
    assert result.returncode == 0, result.stderr
    replies, output = [], result.stdout
    while output:
        header, output = output.split(b"\r\n\r\n", 1)
        length = int(header.split(b":")[1])
        replies.append(json.loads(output[:length]))
        output = output[length:]
    return [r["params"] for r in replies
            if r.get("method") == "textDocument/publishDiagnostics"]


def opened(uri, text, version=1):
    return {"method": "textDocument/didOpen", "params": {"textDocument": {
        "uri": uri, "languageId": "devlang", "version": version, "text": text}}}


def changed(uri, text, version=2):
    return {"method": "textDocument/didChange", "params": {
        "textDocument": {"uri": uri, "version": version}, "contentChanges": [{"text": text}]}}


def closed(uri):
    return {"method": "textDocument/didClose", "params": {"textDocument": {"uri": uri}}}


def semantic(items):
    return [d for d in items if d.get("code") == "semantic-error"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin", required=True)
    binary = Path(parser.parse_args().bin).resolve()
    start = {"id": 1, "method": "initialize", "params": {}}
    end = [{"id": 2, "method": "shutdown"}, {"method": "exit"}]
    with tempfile.TemporaryDirectory(prefix="dev-lsp-semantic-") as temporary:
        root = Path(temporary)
        source = root / "main.dev"
        disk = 'print("disk source stays unchanged")\n'
        source.write_text(disk, encoding="utf-8")
        uri = source.as_uri()
        bad = 'fn main() {\nlet age i64 = "wrong"\nprint(missing)\nif 42 { print(1) }\n}\nmain()\n'
        good = 'fn main() {\nlet age i64 = 18\nprint(age)\nif true { print(1) }\n}\nmain()\n'
        notifications = run(binary, [start, opened(uri, bad), changed(uri, good), *end])
        errors = semantic(notifications[0]["diagnostics"])
        assert len(errors) >= 3, errors
        assert all(d["severity"] == 1 for d in errors)
        assert any("missing" in d["message"] for d in errors)
        assert {d["range"]["start"]["line"] for d in errors} >= {1, 2, 3}
        assert notifications[1]["version"] == 2 and not notifications[1]["diagnostics"]
        assert source.read_text(encoding="utf-8") == disk

        # Semantic ranges use UTF-16 columns, including non-BMP text before the error.
        notifications = run(binary, [start, opened(uri, 'print("😀"); print(missing)'), *end])
        error = semantic(notifications[0]["diagnostics"])[0]
        assert error["range"]["start"]["character"] == 19, error
        assert error["range"]["end"]["character"] == 26, error

        helper = root / "helper.dev"
        helper_disk = 'fn value() str { return "disk" }\n'
        helper.write_text(helper_disk, encoding="utf-8")
        helper_uri = helper.as_uri()
        main_text = 'use "helper"\nlet result i64 = helper.value()\nprint(result)\n'
        notifications = run(binary, [start, opened(uri, main_text),
            opened(helper_uri, 'fn value() i64 { return 42 }'),
            changed(helper_uri, 'fn value() bool { return true }'), closed(helper_uri), *end])
        main_updates = [n for n in notifications if n["uri"] == uri]
        assert len(main_updates) == 4, main_updates
        assert semantic(main_updates[0]["diagnostics"])
        assert not main_updates[1]["diagnostics"]
        assert semantic(main_updates[2]["diagnostics"])
        assert semantic(main_updates[3]["diagnostics"])
        assert helper.read_text(encoding="utf-8") == helper_disk
        assert source.read_text(encoding="utf-8") == disk
    print("PASS: semantic errors, multi-error recovery, UTF-16, fixes, unsaved imports and close refresh")


if __name__ == "__main__":
    main()
