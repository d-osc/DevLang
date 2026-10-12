const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const textmate = require('vscode-textmate');
const oniguruma = require('vscode-oniguruma');
test('interpolation colors expressions and balances nested strings and braces', async () => {
    const wasm = await fs.readFile(require.resolve('vscode-oniguruma/release/onig.wasm'));
    await oniguruma.loadWASM(wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength));
    const registry = new textmate.Registry({
        onigLib: Promise.resolve({ createOnigScanner: patterns => new oniguruma.OnigScanner(patterns), createOnigString: text => new oniguruma.OnigString(text) }),
        loadGrammar: async () => JSON.parse(await fs.readFile(path.join(__dirname, '../syntaxes/devlang.tmLanguage.json'), 'utf8')),
    });
    const grammar = await registry.loadGrammar('source.devlang');
    const tokens = line => grammar.tokenizeLine(line).tokens.map(t => ({text:line.slice(t.startIndex,t.endIndex), scopes:t.scopes}));
    assert.ok(tokens('print("Hello ${name} ${1+2}")').some(t=>t.text==='name' && t.scopes.includes('variable.other.readwrite.devlang')));
    assert.ok(tokens('print("Hello ${name} ${1+2}")').some(t=>t.text==='1' && t.scopes.includes('constant.numeric.devlang')));
    assert.ok(tokens(String.raw`print("literal \${name}")`).every(t=>!t.scopes.includes('meta.interpolation.devlang')));
    const nested=grammar.tokenizeLine('print("${{x: "nested"}.x} tail")');
    assert.ok(nested.tokens.some(t=>'print("${{x: "nested"}.x} tail")'.slice(t.startIndex,t.endIndex).includes('tail') && t.scopes.includes('string.quoted.double.devlang')));
    assert.ok(grammar.tokenizeLine('let n=1',nested.ruleStack).tokens.every(t=>!t.scopes.includes('string.quoted.double.devlang')));
    registry.dispose();
});
