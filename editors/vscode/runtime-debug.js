const path = require('node:path');

function setupRuntimeDebug(vscode, context) {
    context.subscriptions.push(vscode.commands.registerCommand('devlang.debugRuntime', async () => {
        if (!vscode.workspace.isTrusted) {
            vscode.window.showErrorMessage('Trust this workspace before running DevLang code.');
            return;
        }
        const document = vscode.window.activeTextEditor?.document;
        if (!document || document.languageId !== 'devlang' || document.uri.scheme !== 'file') {
            vscode.window.showErrorMessage('Open a saved .dev file to debug its source.');
            return;
        }
        try {
            if (document.isDirty && !(await document.save())) return;
            const executable = vscode.workspace.getConfiguration('devlang', document.uri).get('executablePath', 'd');
            if (!executable.trim()) throw new Error('devlang.executablePath is empty');
            // Launch the executable directly. Never interpolate paths into shell text.
            const terminal = vscode.window.createTerminal({
                name: `DevLang Debug: ${path.basename(document.uri.fsPath)}`,
                shellPath: executable,
                shellArgs: ['run', document.uri.fsPath, '--debug-runtime'],
                cwd: path.dirname(document.uri.fsPath),
                isTransient: true,
            });
            context.subscriptions.push(terminal);
            terminal.show();
        } catch (error) {
            vscode.window.showErrorMessage(`Cannot launch DevLang runtime debugger: ${error.message}. Set devlang.executablePath to a current d executable.`);
        }
    }));
}

module.exports = { setupRuntimeDebug };
