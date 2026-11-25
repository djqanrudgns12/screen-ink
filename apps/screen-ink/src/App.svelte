<script>
  import { onMount } from 'svelte';
  import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
  import { IslandHost, SettingsWindow, TrayPopoverScreen, applyTheme, watchSystemTheme } from '@screen-ink/ui';

  /** @type {{ client: import('@screen-ink/client').InkClient, view: 'popover'|'settings'|'island' }} */
  let { client, view } = $props();

  let pref = $state('system');
  let systemDark = $state(typeof window !== 'undefined' && window.matchMedia?.('(prefers-color-scheme: dark)').matches);
  let shell = $state(null);

  $effect(() => {
    client.getState().then((s) => (pref = s.theme));
    const off = client.subscribe((e) => {
      if (e.type === 'state') pref = e.state.theme;
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
{:else}
  <div class="app-drag-strip" data-tauri-drag-region></div>
  <SettingsWindow {client} onClose={() => client.run('settings.close')} />
{/if}

<style>
  .popover-shell {
    display: inline-block;
    padding: 8px;
  }
</style>
