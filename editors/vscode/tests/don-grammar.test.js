const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const textmate = require('vscode-textmate');
const oniguruma = require('vscode-oniguruma');

test('DON grammar colors keys, references, comments, escapes and multiline strings', async () => {
    const wasm = await fs.readFile(require.resolve('vscode-oniguruma/release/onig.wasm'));
    await oniguruma.loadWASM(wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength));
    const registry = new textmate.Registry({
        onigLib: Promise.resolve({ createOnigScanner: patterns => new oniguruma.OnigScanner(patterns), createOnigString: text => new oniguruma.OnigString(text) }),
        loadGrammar: async () => JSON.parse(await fs.readFile(path.join(__dirname, '../syntaxes/don.tmLanguage.json'), 'utf8')),
    });
    const grammar = await registry.loadGrammar('source.don');
    const manifest = JSON.parse(await fs.readFile(path.join(__dirname,'../package.json'),'utf8'));
    const scopes = manifest.contributes.configurationDefaults['editor.tokenColorCustomizations'].textMateRules.flatMap(rule => rule.scope);
    for (const scope of ['string.quoted.single.don','string.quoted.double.don','string.quoted.triple.don','variable.other.reference.don']) {
        assert.ok(scopes.includes(scope), `Missing code color rule: ${scope}`);
    }
    const tokens = line => grammar.tokenizeLine(line).tokens.map(t => ({text: line.slice(t.startIndex,t.endIndex), scopes:t.scopes}));
    const line = `name: 'Dev' # comment`;
    assert.ok(tokens(line).some(t => t.text === 'name' && t.scopes.includes('support.type.property-name.don')));
    assert.ok(tokens(line).some(t => t.text.includes('Dev') && t.scopes.includes('string.quoted.single.don')));
    assert.ok(tokens(line).some(t => t.text.includes('# comment') && t.scopes.includes('comment.line.don')));
    assert.ok(tokens('rev: @version').some(t => t.text === '@version' && t.scopes.includes('variable.other.reference.don')));
    assert.ok(tokens('rev: "@version"').every(t => !t.scopes.includes('variable.other.reference.don')));
    assert.ok(tokens('count: 1_000').some(t => t.scopes.includes('constant.numeric.don')));
    assert.ok(tokens('ready: true').some(t => t.scopes.includes('constant.language.don')));
    assert.ok(tokens('text: "a\\n"').some(t => t.scopes.includes('constant.character.escape.don')));
    assert.ok(tokens('/* comment */').some(t => t.scopes.includes('comment.block.don')));
    const multiline = grammar.tokenizeLine('text: """hello');
    const continuation = grammar.tokenizeLine('@notAReference', multiline.ruleStack);
    assert.ok(continuation.tokens.every(t => t.scopes.includes('string.quoted.triple.don')));
    registry.dispose();
});
