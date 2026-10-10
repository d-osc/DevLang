const vscode = require('vscode');
const { LanguageClient } = require('vscode-languageclient/node');
const { execFile } = require('node:child_process');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { setupFileIcons } = require('./file-icons');
let client;
let starting;

async function activate(context) {
    setupFileIcons(vscode, context);
    context.subscriptions.push(vscode.languages.registerDocumentFormattingEditProvider(['devlang', 'don'], {
        async provideDocumentFormattingEdits(document, options, token) {
            const version = document.version;
            let directory;
            try {
                directory = await fs.mkdtemp(path.join(os.tmpdir(), 'devlang-format-'));
                const don = document.languageId === 'don';
                const file = path.join(directory, don ? 'document.don' : 'document.dev');
                await fs.writeFile(file, document.getText(), 'utf8');
                const executable = vscode.workspace.getConfiguration('devlang', document.uri).get('executablePath', 'd');
                const formatted = await new Promise((resolve, reject) => {
                    execFile(executable, don ? ['don', 'fmt-source', file] : ['fmt', '--stdout', file], { windowsHide: true, timeout: 30000, maxBuffer: 16 * 1024 * 1024 }, (error, stdout, stderr) => {
                        if (error) reject(new Error(stderr.trim() || error.message));
                        else resolve(stdout);
                    });
                });
                if (token.isCancellationRequested || document.version !== version) return [];
                return [vscode.TextEdit.replace(new vscode.Range(document.positionAt(0), document.positionAt(document.getText().length)), formatted)];
            } catch (error) {
                vscode.window.showErrorMessage(`DevLang/DON formatting failed: ${error.message}. Configure devlang.executablePath to a current d executable with fmt and don fmt-source support.`);
                return [];
            } finally {
                if (directory) await fs.rm(directory, { recursive: true, force: true });
            }
        },
    }));
    const ensureServer = () => {
        if (!starting) starting = startServer();
        return starting;
    };
    context.subscriptions.push(vscode.workspace.onDidOpenTextDocument(document => {
        if (['devlang', 'don'].includes(document.languageId)) void ensureServer();
    }));
    context.subscriptions.push(vscode.commands.registerCommand('devlang.restartServer', async () => {
        if (client) await client.dispose();
        client = undefined;
        starting = undefined;
        await ensureServer();
    }));
    if (vscode.workspace.textDocuments.some(document => ['devlang', 'don'].includes(document.languageId))) await ensureServer();
}

async function startServer() {
    const command = vscode.workspace.getConfiguration('devlang').get('executablePath', 'd');
    client = new LanguageClient('devlang', 'DevLang', {
        command,
        args: ['lsp', '--stdio'],
        options: { cwd: vscode.workspace.workspaceFolders?.[0]?.uri.fsPath },
    }, {
        documentSelector: [{ scheme: 'file', language: 'devlang' }, { scheme: 'file', language: 'don' }, { scheme: 'untitled', language: 'don' }],
    });
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
