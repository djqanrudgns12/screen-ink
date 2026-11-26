// TauriInkClient — 실제 앱. 명령 이름은 tauri-plugin-screen-ink의 `plugin:screen-ink|…`와 1:1.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { encodeAnnotatedCommit } from './index.js';

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
    overlayReady: (session, ui) => cmd('overlay_ready', { session, loadMs: ui?.loadMs, paintMs: ui?.paintMs }),
    captureInput: (input) => cmd('capture_input', { input }),
    pinInfo: (id) => cmd('pin_info', { id }),
    pinAction: (id, action) => cmd('pin_action', { id, action }),
    pinZoom: (id, dy, fx, fy) => cmd('pin_zoom', { id, dy, fx, fy }),
    captureAnnotations: (session, monitorId, items) => cmd('capture_annotations', { session, monitorId, items }),
    // 원본 바이트 IPC(tauri::ipc::Request body) — base64 없음
    captureCommitAnnotated: (meta, rgba) => invoke('plugin:screen-ink|capture_commit_annotated', encodeAnnotatedCommit(meta, rgba)),
    subscribe(handler) {
      const offs = [
        listen('screen-ink://state', (e) => handler({ type: 'state', state: e.payload })),
        listen('screen-ink://island', (e) => handler({ type: 'island', island: e.payload })),
        listen('screen-ink://overlay', (e) => handler({ type: 'overlay', overlay: e.payload })),
        listen('screen-ink://capture', (e) => handler({ type: 'capture', view: e.payload })),
        listen('screen-ink://afterimage', (e) => handler({ type: 'afterimage', afterimage: e.payload })),
        listen('screen-ink://capture-commit', (e) => handler({ type: 'captureCommit', request: e.payload })),
      ];
      return () => offs.forEach((p) => p.then((off) => off()));
    },
  };
}
