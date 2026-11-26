// TauriInkClient — 실제 앱. 명령 이름은 tauri-plugin-screen-ink의 `plugin:screen-ink|…`와 1:1.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { encodeAnnotatedCommit } from './index.js';
import { createEventHub } from './hub.js';

const cmd = (name, args, options) => invoke(`plugin:screen-ink|${name}`, args, options);

/**
 * 판서 내보내기: 원본 바이트를 그대로 본문으로(base64 없음), 크기·동작은 머리글로(draw_export).
 * Uint8ClampedArray(getImageData)도 복사 없이 Uint8Array로 본다.
 * @param {'copy'|'save'} action
 * @param {ArrayBuffer|ArrayBufferView} bytes
 * @param {import('./index.js').DrawExportMeta} meta
 */
function exportRequest(action, bytes, meta) {
  const body = bytes instanceof ArrayBuffer ? new Uint8Array(bytes) : new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  /** @type {Record<string, string>} */
  const headers = { 'x-ink-action': action, 'x-ink-w': String(meta.w), 'x-ink-h': String(meta.h) };
  if (meta.session != null) headers['x-ink-session'] = String(meta.session);
  if (meta.format) headers['x-ink-format'] = meta.format;
  return { body, headers };
}

/** @returns {import('./index.js').InkClient} */
export function createTauriClient() {
  /** @type {ReturnType<typeof createEventHub> | null} */
  let hub = null;
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
    drawReady: (session, paintMs) => cmd('draw_ready', { session, paintMs }),
    drawFinish: (payload) => cmd('draw_finish', { payload }),
    drawRestore: (session) => cmd('draw_restore', { session }),
    drawExport: (action, bytes, meta) => {
      const { body, headers } = exportRequest(action, bytes, meta);
      return cmd('draw_export', body, { headers });
    },
    drawIsland: (patch) => cmd('draw_island', { patch }),
    drawPenMetrics: (sample) => cmd('draw_pen_metrics', { sample }),
    zoomInput: (input) => cmd('zoom_input', { input }),
    captureAnnotations: (session, monitorId, items) => cmd('capture_annotations', { session, monitorId, items }),
    // 원본 바이트 IPC(tauri::ipc::Request body) — base64 없음
    captureCommitAnnotated: (meta, rgba) => invoke('plugin:screen-ink|capture_commit_annotated', encodeAnnotatedCommit(meta, rgba)),
    subscribe(handler) {
      // 창마다 듣기는 한 번만(hub.js — 구독마다 listen을 걸면 등록 전에 온 사건을 놓친다)
      hub ??= createEventHub(listen);
      return hub.add(handler);
    },
  };
}
