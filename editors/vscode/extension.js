const vscode = require('vscode');
const { LanguageClient } = require('vscode-languageclient/node');
let client;

async function activate(context) {
    const command = vscode.workspace.getConfiguration('devlang').get('executablePath', 'd');
    client = new LanguageClient('devlang', 'DevLang', {
        command,
        args: ['lsp', '--stdio'],
        options: { cwd: vscode.workspace.workspaceFolders?.[0]?.uri.fsPath },
    }, {
        documentSelector: [{ scheme: 'file', language: 'devlang' }],
    });
    context.subscriptions.push(vscode.commands.registerCommand('devlang.restartServer', async () => {
        await client.stop();
        await client.start();
    }));
    await client.start();
}
async function deactivate() {
    if (client) await client.stop();
}
module.exports = { activate, deactivate };
