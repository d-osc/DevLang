const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

test('startup configures icons without starting LSP; opening DevLang starts once', async () => {
    let opened, restart, icons = 0, formatter = 0, executable = 'd', disposed = 0;
    const starts = [];
    const vscode = {
        workspace: { textDocuments: [], getConfiguration: () => ({ get: () => executable }),
            onDidOpenTextDocument: listener => { opened = listener; return {}; } },
        languages: { registerDocumentFormattingEditProvider: () => { formatter++; return {}; } },
        commands: { registerCommand: (id, callback) => { restart = callback; return {}; } },
        window: { showErrorMessage: message => assert.fail(message) },
    };
    class LanguageClient {
        constructor(id, name, options) { this.command = options.command; }
        async start() { starts.push(this.command); }
        async dispose() { disposed++; }
        getFeature() { return { clear() {} }; }
    }
    const module = { exports: {} };
    vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../extension.js'), 'utf8'), {
        module, console, require(name) {
            if (name === 'vscode') return vscode;
            if (name === 'vscode-languageclient/node') return { LanguageClient };
            if (name === './file-icons') return { setupFileIcons: () => icons++ };
            return require(name);
        },
    });
    await module.exports.activate({ subscriptions: [] });
    assert.equal(icons, 1); assert.equal(formatter, 1); assert.deepEqual(starts, []);
    opened({ languageId: 'javascript' });
    opened({ languageId: 'devlang' });
    opened({ languageId: 'devlang' });
    await new Promise(setImmediate);
    assert.deepEqual(starts, ['d']);
    executable = 'new-d';
    await restart();
    assert.deepEqual(starts, ['d', 'new-d']); assert.equal(disposed, 1);
});
