// TauriInkClient — 실제 앱. 명령 이름은 tauri-plugin-screen-ink의 `plugin:screen-ink|…`와 1:1.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const cmd = (name, args) => invoke(`plugin:screen-ink|${name}`, args);

/** @returns {import('./index.js').InkClient} */
export function createTauriClient() {
  return {
    getState: () => cmd('get_state'),
    updateSetting: (path, value) => cmd('update_setting', { path, value }),
    previewShortcut: (id, combo) => cmd('preview_shortcut', { id, combo }),
    setShortcut: (id, combo) => cmd('set_shortcut', { id, combo }),
    resetShortcutGroup: (group) => cmd('reset_shortcut_group', { group }),
    undo: () => cmd('undo'),
    setHotkeysEnabled: (enabled) => cmd('set_hotkeys_enabled', { enabled }),
    run: (action) => cmd('run', { action }),
    previewImport: () => cmd('preview_import'),
    applyImport: (path) => cmd('apply_import', { path }),
    resetAll: () => cmd('reset_all'),
    subscribe(handler) {
      const offs = [
        listen('screen-ink://state', (e) => handler({ type: 'state', state: e.payload })),
        listen('screen-ink://island', (e) => handler({ type: 'island', island: e.payload })),
      ];
      return () => offs.forEach((p) => p.then((off) => off()));
    },
  };
}
