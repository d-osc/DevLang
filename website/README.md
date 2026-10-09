# DevLang documentation website

Static Thai learning guide, complete reference documentation, executable examples and a downloadable AI skill. No browser interpreter is advertised or included.

Build from the repository root:

```sh
python -m pip install --target website/.build-deps -r website/requirements.txt
python website/build.py
python -m http.server 4173 --directory website/dist --bind 127.0.0.1
```

Edit `content/guide.md` for the Thai guide. Reference pages are generated from repository docs. Examples are read from `examples/`; the build captures output with the actual source runtime. The website includes a ZIP generated from `skills/devlang`, `llms.txt`, plain Markdown documents and source downloads.

`dist` is a complete static deployment directory. GitHub Pages publishes it at https://d-osc.github.io/DevLang/ through `.github/workflows/pages.yml`. Changes to website source, reference docs, examples or the AI skill on `main` trigger deployment; the workflow can also be started manually. It builds with `--skip-validation`, checking example source hashes against the validated outputs in `content/example-outputs.json`. If an example changes, run a validated local build and commit its refreshed snapshot before publishing.

Assets and downloads use relative URLs, and documentation navigation uses hash routes, so the site works under the `/DevLang/` project path. The Sites hosting manifest retains the separate hosted Site identity; use the Sites source workflow for that host.

Standalone Site clones use the tracked snapshots in `content/materials` and `content/example-outputs.json`. Run `python build.py --skip-validation` to rebuild with those outputs, or supply `--d /path/to/d` for fresh runtime validation. In the DevLang checkout, builds refresh the snapshots from the current repository docs/examples/skill.
