const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const textmate = require('vscode-textmate');
const oniguruma = require('vscode-oniguruma');

const manifest = require('../package.json');
const devlang = manifest.contributes.languages.find(l => l.id === 'devlang');

test('firstLine detects dev and d shebangs only', () => {
    const first = new RegExp(devlang.firstLine);
    for (const line of ['#!/usr/bin/env dev', '#!/usr/bin/env d', '#!/usr/bin/env -S dev', '#!/usr/local/bin/dev', '#!/usr/bin/d --timings', '#! /usr/bin/env dev'])
        assert.ok(first.test(line), line);
    for (const line of ['#!/usr/bin/env node', '#!/bin/bash', '#!/usr/bin/env devtool', '# dev', 'let d = 1'])
        assert.ok(!first.test(line), line);
});

test('shebang line is tokenized as a comment on line one only', async () => {
    const wasm = await fs.readFile(require.resolve('vscode-oniguruma/release/onig.wasm'));
    await oniguruma.loadWASM(wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength));
    const registry = new textmate.Registry({
        onigLib: Promise.resolve({ createOnigScanner: p => new oniguruma.OnigScanner(p), createOnigString: t => new oniguruma.OnigString(t) }),
        loadGrammar: async () => JSON.parse(await fs.readFile(path.join(__dirname, '../syntaxes/devlang.tmLanguage.json'), 'utf8')),
    });
    const grammar = await registry.loadGrammar('source.devlang');
    const first = grammar.tokenizeLine('#!/usr/bin/env dev');
    assert.ok(first.tokens.every(t => t.scopes.includes('comment.line.shebang.devlang')));
    const second = grammar.tokenizeLine('#!/usr/bin/env dev', first.ruleStack);
    assert.ok(second.tokens.every(t => !t.scopes.includes('comment.line.shebang.devlang')));
    registry.dispose();
});
