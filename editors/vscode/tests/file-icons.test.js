const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { configureMaterialIcons } = require('../file-icons');

async function fixture(t, options = {}) {
    const root = await fs.mkdtemp(path.join(os.tmpdir(), 'devlang-icons-'));
    t.after(() => fs.rm(root, { recursive: true, force: true }));
    const material = { extensionPath: path.join(root, 'material-icon-theme-1.0') };
    let global = options.associations || { '*.other': 'javascript' };
    let updates = 0;
    const state = { enabled: true, theme: 'material-icon-theme', ...options };
    const context = { extensionPath: path.resolve(__dirname, '..'), extensionUri: { scheme: 'file' } };
    const api = {
        extensions: { getExtension: () => state.absent ? undefined : material },
        ConfigurationTarget: { Global: 1 },
        workspace: { getConfiguration(section) {
            if (section === 'devlang') return { get: () => state.enabled };
            if (section === 'workbench') return { get: () => state.theme };
            return {
                get: () => global,
                inspect: () => ({ globalValue: global }),
                update: async (key, value, target) => {
                    assert.equal(key, 'files.associations'); assert.equal(target, 1);
                    global = value; updates++;
                },
            };
        } },
    };
    return { api, context, state, icon: path.join(root, 'devlang-file-icons/file-dev.svg'),
        global: () => global, updates: () => updates };
}

test('adds only .dev, copies SVG, preserves associations and is idempotent', async t => {
    const f = await fixture(t);
    await configureMaterialIcons(f.api, f.context);
    assert.deepEqual(f.global(), { '*.other': 'javascript', '*.dev': '../../devlang-file-icons/file-dev' });
    assert.deepEqual(await fs.readFile(f.icon), await fs.readFile(path.join(f.context.extensionPath, 'images/file-dev.svg')));
    await configureMaterialIcons(f.api, f.context);
    assert.equal(f.updates(), 1);
});
test('respects custom .dev and **.dev mappings', async t => {
    for (const associations of [{ '*.dev': 'custom' }, { '**.dev': 'custom' }]) {
        const f = await fixture(t, { associations });
        await configureMaterialIcons(f.api, f.context);
        assert.equal(f.updates(), 0);
        await assert.rejects(fs.stat(f.icon), { code: 'ENOENT' });
    }
});
test('does nothing for other themes or missing Material Icon Theme', async t => {
    for (const options of [{ theme: 'another-theme' }, { absent: true }]) {
        const f = await fixture(t, options);
        await configureMaterialIcons(f.api, f.context);
        assert.equal(f.updates(), 0);
        await assert.rejects(fs.stat(f.icon), { code: 'ENOENT' });
    }
});
test('disabling removes our association and keeps user mappings', async t => {
    const f = await fixture(t);
    await configureMaterialIcons(f.api, f.context);
    f.state.enabled = false;
    await configureMaterialIcons(f.api, f.context);
    assert.deepEqual(f.global(), { '*.other': 'javascript' });
    const custom = await fixture(t, { enabled: false, associations: { '*.dev': 'custom' } });
    await configureMaterialIcons(custom.api, custom.context);
    assert.equal(custom.updates(), 0);
});
