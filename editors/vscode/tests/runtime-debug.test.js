const { test } = require('node:test');
const assert = require('node:assert/strict');
const { setupRuntimeDebug } = require('../runtime-debug');

function fixture() {
    const state = { errors: [], launches: [], shown: 0, saved: 0 };
    const document = { languageId: 'devlang', uri: { scheme: 'file', fsPath: '/project with spaces/$sample.dev' }, isDirty: true,
        async save() { state.saved++; return true; } };
    const vscode = {
        workspace: { isTrusted: true, getConfiguration: () => ({ get: () => '/tools with spaces/d' }) },
        window: { activeTextEditor: { document }, showErrorMessage: message => state.errors.push(message),
            createTerminal: options => { state.launches.push(options); return { show() { state.shown++; }, dispose() {} }; } },
        commands: { registerCommand: (id, callback) => { assert.equal(id, 'devlang.debugRuntime'); state.run = callback; return { dispose() {} }; } },
    };
    const context = { subscriptions: [] };
    setupRuntimeDebug(vscode, context);
    return { state, vscode, document, context };
}

test('runtime command saves source and passes paths as executable arguments', async () => {
    const { state, context } = fixture();
    await state.run();
    assert.equal(state.saved, 1);
    assert.equal(state.shown, 1);
    assert.equal(context.subscriptions.length, 2);
    assert.deepEqual(state.launches[0], { name: 'DevLang Debug: $sample.dev', shellPath: '/tools with spaces/d',
        shellArgs: ['run', '/project with spaces/$sample.dev', '--debug-runtime'], cwd: '/project with spaces', isTransient: true });
});

test('reject untrusted, unsaved or non-DevLang documents', async () => {
    for (const change of [f => f.vscode.workspace.isTrusted = false, f => f.document.uri.scheme = 'untitled',
        f => f.document.languageId = 'don', f => f.vscode.window.activeTextEditor = undefined]) {
        const f = fixture(); change(f); await f.state.run();
        assert.equal(f.state.launches.length, 0); assert.equal(f.state.errors.length, 1);
    }
});

test('cancelled save and launch failures do not run source', async () => {
    const f = fixture(); f.document.save = async () => false; await f.state.run();
    assert.equal(f.state.launches.length, 0);
    f.document.isDirty = false;
    f.vscode.window.createTerminal = () => { throw new Error('missing executable'); };
    await f.state.run(); assert.match(f.state.errors[0], /missing executable/);
});
