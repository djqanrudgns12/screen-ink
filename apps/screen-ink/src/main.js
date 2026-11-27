// 앱 진입점 — 창 하나에 화면 하나(?view=popover|settings|island|overlay|pin). UI는 InkClient(Tauri)만 안다.
import './app.css';
import { mount } from 'svelte';
import { createTauriClient } from '@screen-ink/client/tauri';
import { invoke } from '@tauri-apps/api/core';
import App from './App.svelte';

// 화면 오류를 앱 로그로(진단 — 개발 도구를 못 여는 사용자 PC에서도 로그 하나로 본다)
const uiLog = (level, message) => invoke('plugin:screen-ink|ui_log', { level, message: String(message).slice(0, 400) }).catch(() => {});
window.addEventListener('error', (e) => uiLog('error', `${e.message} @ ${e.filename?.split('/').pop() ?? ''}:${e.lineno ?? 0}`));
window.addEventListener('unhandledrejection', (e) => uiLog('error', `unhandled: ${e.reason?.stack ?? e.reason}`));
const consoleError = console.error.bind(console);
console.error = (...args) => {
  consoleError(...args);
  uiLog('warn', args.map((a) => (a instanceof Error ? a.stack ?? a.message : String(a))).join(' '));
};

const params = new URLSearchParams(location.search);
const view = params.get('view') ?? 'settings';
document.documentElement.dataset.view = view;

mount(App, {
  target: document.getElementById('app'),
  props: { client: createTauriClient(), view, firstRun: params.get('first') === '1', monitorId: Number(params.get('monitor') ?? 0), pinId: Number(params.get('pin') ?? 0) },
});

// 글자 입력 진단(R2-TXT-0, 개발 실행만): 글 상자가 초점을 받으면 맥 입력 상태를 앱 기록에
if (import.meta.env.DEV) {
  window.addEventListener('ink-debug-input', (e) => {
    void invoke('plugin:screen-ink|debug_input', { place: String(e.detail ?? '') }).catch(() => {});
  });
}
