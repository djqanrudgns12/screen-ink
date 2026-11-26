// 앱 진입점 — 창 하나에 화면 하나(?view=popover|settings|island|overlay|pin). UI는 InkClient(Tauri)만 안다.
import './app.css';
import { mount } from 'svelte';
import { createTauriClient } from '@screen-ink/client/tauri';
import App from './App.svelte';

const params = new URLSearchParams(location.search);
const view = params.get('view') ?? 'settings';
document.documentElement.dataset.view = view;

mount(App, {
  target: document.getElementById('app'),
  props: { client: createTauriClient(), view, firstRun: params.get('first') === '1', monitorId: Number(params.get('monitor') ?? 0), pinId: Number(params.get('pin') ?? 0) },
});
