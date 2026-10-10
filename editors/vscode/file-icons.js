const fs = require('node:fs/promises');
const path = require('node:path');

// Use Material Icon Theme's supported custom-SVG setting, never edit its theme.
async function configureMaterialIcons(vscode, context) {
    const material = vscode.extensions.getExtension('PKief.material-icon-theme');
    if (!material || context.extensionUri?.scheme !== 'file') return;
    const directory = path.join(path.dirname(material.extensionPath), 'devlang-file-icons');
    const config = vscode.workspace.getConfiguration('material-icon-theme');
    const global = { ...(config.inspect('files.associations')?.globalValue || {}) };
    const enabled = vscode.workspace.getConfiguration('devlang').get('autoFileIcon', true);
    if (enabled && vscode.workspace.getConfiguration('workbench').get('iconTheme') !== 'material-icon-theme') return;
    const effective = config.get('files.associations', {});
    let changed = false;
    for (const extension of ['dev', 'don']) {
        const icon = path.join(directory, `file-${extension}.svg`);
        const association = path.relative(path.join(material.extensionPath, 'dist'), icon.slice(0, -4)).split(path.sep).join('/');
        const key = `*.${extension}`;
        if (!enabled) {
            if (global[key] === association) { delete global[key]; changed = true; }
            continue;
        }
        // Preserve explicit user mappings independently for each language.
        if (effective[`**.${extension}`] || (effective[key] && effective[key] !== association)) continue;
        const source = await fs.readFile(path.join(context.extensionPath, 'images', `file-${extension}.svg`));
        await fs.mkdir(directory, { recursive: true });
        const existing = await fs.readFile(icon).catch(error => {
            if (error.code !== 'ENOENT') throw error;
            return undefined;
        });
        if (!existing || !existing.equals(source)) await fs.writeFile(icon, source);
        if (global[key] !== association) { global[key] = association; changed = true; }
    }
    if (changed) await config.update('files.associations', global, vscode.ConfigurationTarget.Global);
}

function setupFileIcons(vscode, context) {
    let pending = Promise.resolve();
    const refresh = () => {
        pending = pending.then(() => configureMaterialIcons(vscode, context))
            .catch(error => console.warn('DevLang file icon integration:', error.message));
    };
    context.subscriptions.push(vscode.workspace.onDidChangeConfiguration(event => {
        if (event.affectsConfiguration('workbench.iconTheme') || event.affectsConfiguration('devlang.autoFileIcon')) refresh();
    }));
    context.subscriptions.push(vscode.extensions.onDidChange(refresh));
    refresh();
}

module.exports = { configureMaterialIcons, setupFileIcons };
