<script>
  import { onMount, untrack } from 'svelte';
  import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
  import { listen } from '@tauri-apps/api/event';
  import { CaptureOverlay, DrawOverlay, IslandHost, PinView, SettingsWindow, TrayPopoverScreen, applyTheme, watchSystemTheme } from '@screen-ink/ui';

  /** @type {{ client: import('@screen-ink/client').InkClient, view: 'popover'|'settings'|'island'|'overlay'|'pin', firstRun?: boolean, monitorId?: number, pinId?: number }} */
  let { client, view, firstRun = false, monitorId = 0, pinId = 0 } = $props();

  let pref = $state('system');
  let systemDark = $state(typeof window !== 'undefined' && window.matchMedia?.('(prefers-color-scheme: dark)').matches);
  let shell = $state(null);
  // 첫 실행 시트: 창을 열 때 ?first=1, 이미 열린 창이면 이벤트로 다시(일반 > [다시 보기])
  let showFirstRun = $state(untrack(() => firstRun));
  let firstRunKey = $state(0);
  // 오버레이 창: 지금 모드와 마지막 판서 세션 — 판서면 그 모니터 창만 DrawOverlay, 아니면 CaptureOverlay(창 재사용, A §2.5)
  /** @type {import('@screen-ink/client').Mode} */
  let mode = $state({ mode: 'idle' });
  /** @type {import('@screen-ink/client').DrawSession | null} */
  let drawSession = $state(null);
  let drawHere = $derived(
    view === 'overlay' && mode.mode === 'draw' && drawSession && drawSession.session === mode.session && drawSession.monitor.id === monitorId
      ? drawSession
      : null,
  );

  $effect(() => {
    client.getState().then((s) => {
      pref = s.theme;
      if (s.mode) mode = s.mode;
    });
    const off = client.subscribe((e) => {
      if (e.type === 'state') {
        pref = e.state.theme;
        if (e.state.mode) mode = e.state.mode;
      } else if (e.type === 'draw') {
        // 세션이 상태보다 먼저 온다 — 상태가 draw로 바뀌는 순간 이미 쥐고 있다
        drawSession = e.draw;
      }
    });
    const unwatch = watchSystemTheme((dark) => (systemDark = dark));
    return () => {
      off();
      unwatch();
    };
  });

  $effect(() => {
    applyTheme(document.documentElement, pref, systemDark);
  });

  onMount(() => {
    if (view !== 'settings') return;
    const un = listen('screen-ink://first-run', () => {
      showFirstRun = true;
      firstRunKey += 1;
    });
    return () => un.then((f) => f());
  });

  // 팝오버: 카드 크기에 창을 맞춘다(투명 여백 8px).
  onMount(() => {
    if (view !== 'popover' || !shell) return;
    const win = getCurrentWindow();
    const fit = () => {
      const card = shell.firstElementChild;
      if (!card) return;
      const r = card.getBoundingClientRect();
      win.setSize(new LogicalSize(Math.ceil(r.width) + 16, Math.ceil(r.height) + 16));
    };
    const ro = new ResizeObserver(fit);
    ro.observe(shell);
    fit();
    return () => ro.disconnect();
  });

  // 섬: 화면에서 섬이 사라지면 창도 숨긴다(투명 창이 클릭을 가로막지 않게).
  onMount(() => {
    if (view !== 'island' || !shell) return;
    const mo = new MutationObserver(() => {
      if (!shell.querySelector('.ink-island-host')) client.run('island.hidden');
    });
    mo.observe(shell, { childList: true, subtree: true });
    return () => mo.disconnect();
  });
</script>

{#if view === 'popover'}
  <div class="popover-shell" bind:this={shell}>
    <TrayPopoverScreen {client} onClose={() => client.run('popover.close')} />
  </div>
{:else if view === 'island'}
  <div bind:this={shell}>
    <IslandHost {client} />
  </div>
{:else if view === 'overlay'}
  {#if drawHere}
    {#key drawHere.session}
      <DrawOverlay {client} {monitorId} session={drawHere} />
    {/key}
  {:else}
    <CaptureOverlay {client} {monitorId} />
  {/if}
{:else if view === 'pin'}
  <PinView {client} {pinId} />
{:else}
  {#key firstRunKey}
    <SettingsWindow {client} firstRun={showFirstRun} onClose={() => client.run('settings.close')} />
  {/key}
{/if}

<style>
  .popover-shell {
    display: inline-block;
    padding: 8px;
  }
</style>
