const vscode = require('vscode');
const { LanguageClient } = require('vscode-languageclient/node');
const { execFile } = require('node:child_process');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
let client;

async function activate(context) {
    context.subscriptions.push(vscode.languages.registerDocumentFormattingEditProvider('devlang', {
        async provideDocumentFormattingEdits(document, options, token) {
            const version = document.version;
            let directory;
            try {
                directory = await fs.mkdtemp(path.join(os.tmpdir(), 'devlang-format-'));
                const file = path.join(directory, 'document.dev');
                await fs.writeFile(file, document.getText(), 'utf8');
                const executable = vscode.workspace.getConfiguration('devlang').get('executablePath', 'd');
                const formatted = await new Promise((resolve, reject) => {
                    execFile(executable, ['fmt', '--stdout', file], { windowsHide: true, timeout: 30000, maxBuffer: 16 * 1024 * 1024 }, (error, stdout, stderr) => {
                        if (error) reject(new Error(stderr.trim() || error.message));
                        else resolve(stdout);
                    });
                });
                if (token.isCancellationRequested || document.version !== version) return [];
                return [vscode.TextEdit.replace(new vscode.Range(document.positionAt(0), document.positionAt(document.getText().length)), formatted)];
            } catch (error) {
                vscode.window.showErrorMessage(`DevLang formatting failed: ${error.message}. Configure devlang.executablePath to a current d executable with fmt support.`);
                return [];
            } finally {
                if (directory) await fs.rm(directory, { recursive: true, force: true });
            }
        },
    }));
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
        client.getFeature('textDocument/formatting').clear();
    }));
    try {
        await client.start();
        client.getFeature('textDocument/formatting').clear();
    }
    catch (error) { vscode.window.showErrorMessage(`DevLang language server failed: ${error.message}. Set devlang.executablePath to the latest d executable.`); }
}
async function deactivate() {
    if (client) await client.stop();
}
module.exports = { activate, deactivate };
