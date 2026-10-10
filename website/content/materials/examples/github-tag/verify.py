"""Exercise this example using local Git tags; no GitHub credentials needed."""
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tomllib

d = Path(sys.argv[1]).resolve()
base = Path(__file__).resolve().parent
with tempfile.TemporaryDirectory(prefix="dev-github-tag-") as temporary:
    root = Path(temporary)
    lib, app = root / "library", root / "app"
    shutil.copytree(base / "library", lib)
    shutil.copytree(base / "app", app)
    def git(*args):
        subprocess.run(["git", "-C", str(lib), *args], check=True, capture_output=True)
    git("init")
    git("add", ".")
    git("-c", "user.name=Example Test", "-c", "user.email=test@example.com", "commit", "-m", "math 1.2.0")
    git("tag", "v1.2.0")
    def command(*args):
        result = subprocess.run([str(d), "-C", str(app), *args], check=True, capture_output=True, text=True, timeout=45)
        return result.stdout
    for selector in ({"tag": "v1.2.0"}, {"version": "^1.2"}):
        manifest = {"package": {"name": "github_tag_app"}, "dependencies": {"math": {"git": lib.as_uri(), **selector}}}
        (app / "package.don").write_text(json.dumps(manifest), encoding="utf-8")
        command("pkg", "install")
        assert command("run").strip() == "42"
        assert command("run", "--engine", "ast").strip() == "42"
        command("pkg", "install", "--locked")
    command("pkg", "add", "math", "--git", lib.as_uri(), "--tag", "v1.2.0")
    assert command("run").strip() == "42"
    lock = tomllib.loads((app / "dev.lock").read_text(encoding="utf-8"))
    assert lock["packages"]["math"]["source"]["tag"] == "v1.2.0"
    assert "rev" not in lock["packages"]["math"]["source"]
    print("PASS: exact Git tag, SemVer tag, both engines and locked installs")
