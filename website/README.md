# DevLang documentation website

Static Thai learning guide, complete reference documentation, executable examples and a downloadable AI skill. No browser interpreter is advertised or included.

Build from the repository root:

```sh
python -m pip install --target website/.build-deps -r website/requirements.txt
python website/build.py
python -m http.server 4173 --directory website/dist --bind 127.0.0.1
```

Edit `content/guide.md` for the Thai guide. Reference pages are generated from repository docs. Examples are read from `examples/`; the build captures output with the actual source runtime. The website includes a ZIP generated from `skills/devlang`, `llms.txt`, plain Markdown documents and source downloads.

`dist` is a complete static deployment directory and can also be served by GitHub Pages or any static host. The Sites hosting manifest records the hosted Site identity. Use the Sites source workflow for deployment.

Standalone Site clones use the tracked snapshots in `content/materials` and `content/example-outputs.json`. Run `python build.py --skip-validation` to rebuild with those outputs, or supply `--d /path/to/d` for fresh runtime validation. In the DevLang checkout, builds refresh the snapshots from the current repository docs/examples/skill.
