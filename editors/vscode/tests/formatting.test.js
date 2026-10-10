const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

test('Format Document routes unsaved DON to the real lossless formatter and rejects stale edits', async () => {
    const binary = process.env.DEVLANG_TEST_BIN || path.resolve(__dirname, '../../../out/core-io/release/d.exe');
    assert.ok(fs.existsSync(binary), `Build d first or set DEVLANG_TEST_BIN: ${binary}`);
    let provider, selector;
    const errors = [];
    class Range { constructor(start, end) { this.start = start; this.end = end; } }
    const vscode = {
        Range, TextEdit: { replace: (range, newText) => ({range, newText}) },
        workspace: { textDocuments: [], getConfiguration: () => ({get: () => binary}), onDidOpenTextDocument: () => ({}) },
        languages: { registerDocumentFormattingEditProvider: (selected, registered) => {selector=selected;provider=registered;return {};} },
        commands: {registerCommand: () => ({})}, window: {showErrorMessage: message => errors.push(message)},
    };
    const module = {exports:{}};
    vm.runInNewContext(fs.readFileSync(path.join(__dirname,'../extension.js'),'utf8'), {
        module, console, require(name) {
            if (name === 'vscode') return vscode;
            if (name === 'vscode-languageclient/node') return {LanguageClient: class {}};
            if (name === './file-icons') return {setupFileIcons() {}};
            return require(name);
        },
    });
    await module.exports.activate({subscriptions:[]});
    assert.ok(selector.includes('don') && selector.includes('devlang'));
    const text = "// keep\nversion='v1'\nref:@version\nobj:{enabled:true}\n";
    const document = {languageId:'don',version:1,uri:{scheme:'untitled'},getText:()=>text,positionAt:value=>value};
    const edits = await provider.provideDocumentFormattingEdits(document,{}, {isCancellationRequested:false});
    assert.equal(edits.length,1);
    assert.ok(edits[0].newText.includes('// keep') && edits[0].newText.includes('ref: @version'));
    assert.equal(edits[0].range.end,text.length);
    const stale = provider.provideDocumentFormattingEdits(document,{}, {isCancellationRequested:false});
    document.version++;
    assert.equal((await stale).length,0);
    assert.equal((await provider.provideDocumentFormattingEdits(document,{}, {isCancellationRequested:true})).length,0);
    assert.equal((await provider.provideDocumentFormattingEdits({...document,getText:()=> 'ref: @missing'},{},{isCancellationRequested:false})).length,0);
    assert.ok(errors.pop().includes('missing reference target'));
    const dev = await provider.provideDocumentFormattingEdits({...document,languageId:'devlang',getText:()=> 'fn main() {\nprint(42)\n}\nmain()'},{},{isCancellationRequested:false});
    assert.equal(dev.length,1);assert.ok(dev[0].newText.includes('    print(42)'));
    assert.equal(errors.length,0);
});
