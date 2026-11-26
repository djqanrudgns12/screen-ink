<script>
  import AppIcon from '../icons/AppIcon.svelte';
  import TrayPopover from '../components/TrayPopover.svelte';
  import MenuRow from '../components/MenuRow.svelte';
  import Switch from '../components/Switch.svelte';
  import Keycap from '../components/Keycap.svelte';

  /**
   * S01 트레이 팝오버 화면(F03).
   * @type {{
   *   client: import('@screen-ink/client').InkClient,
   *   onClose?: () => void,
   *   class?: string,
   * }}
   */
  let { client, onClose = undefined, class: className = '' } = $props();

  let state = $state(null);

  $effect(() => {
    let alive = true;
    client.getState().then((s) => {
      if (alive) state = s;
    });
    const off = client.subscribe((e) => {
      if (e.type === 'state') state = e.state;
    });
    return () => {
      alive = false;
      off();
    };
  });

  function comboOf(id) {
    return state?.shortcuts?.find((s) => s.id === id)?.combo ?? '';
  }

  function formatElapsed(ms) {
    const totalSec = Math.max(0, Math.floor((ms || 0) / 1000));
    const m = String(Math.floor(totalSec / 60)).padStart(2, '0');
    const s = String(totalSec % 60).padStart(2, '0');
    return `${m}:${s}`;
  }

  let platform = $derived(state?.platform ?? 'windows');
  let hotkeysEnabled = $derived(state?.hotkeysEnabled ?? true);
  let recording = $derived(Boolean(state?.recording?.active));
  let pinnedCount = $derived(state?.pinnedCount ?? 0);
  // 캡처 핀(R-CAP-29)
  let pinCount = $derived(state?.pins ?? 0);
  let conflictCount = $derived(state?.conflicts?.count ?? 0);
  let settingsCombo = $derived(platform === 'macos' ? 'Cmd+,' : 'Ctrl+,');

  function runAction(action) {
    client.run(action);
  }
</script>

<TrayPopover label="슉쇽 메뉴" {onClose} class={className}>
  {#snippet header()}
    <AppIcon size={24} />
    <span class="ink-tray-brand">슉쇽</span>
    {#if recording}
      <span class="ink-tray-status" data-tone="danger">
        <i class="ink-tray-dot" data-tone="danger"></i>
        <span>녹화 중 <span class="ink-mono">{formatElapsed(state?.recording?.elapsedMs)}</span></span>
      </span>
    {:else if !hotkeysEnabled}
      <span class="ink-tray-status" data-tone="off">
        <i class="ink-tray-dot" data-tone="off"></i>
        <span>단축키 꺼짐</span>
      </span>
    {:else if conflictCount > 0}
      <button
        type="button"
        class="ink-tray-status ink-tray-status-btn ink-focus"
        data-tone="attn"
        onclick={() => runAction('settings.open')}
      >
        <i class="ink-tray-dot" data-tone="attn"></i>
        <span>키 {conflictCount}개 겹침</span>
      </button>
    {:else}
      <span class="ink-tray-status" data-tone="ok">
        <i class="ink-tray-dot" data-tone="ok"></i>
        <span>준비됨</span>
      </span>
    {/if}
  {/snippet}

  <p class="ink-tray-hd">캡처·녹화</p>
  <MenuRow
    icon="region"
    label="범위 캡처"
    combo={comboOf('region')}
    {platform}
    keyDim={!hotkeysEnabled}
    onclick={() => runAction('capture.region')}
  />
  <MenuRow
    icon="window"
    label="창 캡처"
    combo={comboOf('window')}
    {platform}
    keyDim={!hotkeysEnabled}
    onclick={() => runAction('capture.window')}
  />
  <MenuRow
    icon="record"
    label={recording ? '녹화 중지' : '화면 녹화'}
    combo={comboOf('record')}
    {platform}
    keyDim={!hotkeysEnabled}
    onclick={() => runAction('record.toggle')}
  />

  <p class="ink-tray-hd">화면 도구</p>
  <MenuRow
    icon="draw"
    label="판서"
    combo={comboOf('draw')}
    {platform}
    keyDim={!hotkeysEnabled}
    onclick={() => runAction('draw')}
  />
  <MenuRow
    icon="zoom"
    label="실시간 확대"
    combo={comboOf('zoom')}
    {platform}
    keyDim={!hotkeysEnabled}
    onclick={() => runAction('zoom')}
  />
  <MenuRow
    icon="board"
    label="칠판"
    combo={comboOf('board')}
    {platform}
    keyDim={!hotkeysEnabled}
    onclick={() => runAction('board')}
  />
  {#if platform !== 'macos'}
    <div class="ink-pin-row">
      <MenuRow
        icon="pin"
        label="창 항상 위"
        combo={pinnedCount > 0 ? '' : comboOf('topmost')}
        {platform}
        keyDim={!hotkeysEnabled}
        onclick={() => runAction('pin.toggle')}
      >
        {#snippet trailing()}
          {#if pinnedCount > 0}
            <span class="ink-pin-count ink-mono">{pinnedCount}</span>
          {/if}
        {/snippet}
      </MenuRow>
      {#if pinnedCount > 0}
        <button
          type="button"
          class="ink-pin-release ink-focus"
          onclick={() => runAction('pin.releaseAll')}
        >
          모두 풀기
        </button>
      {/if}
    </div>
  {/if}

  {#if pinCount > 0}
    <div class="ink-pin-row">
      <MenuRow icon="pin" label="고정한 캡처" combo="" {platform} onclick={() => runAction('pins.closeAll')}>
        {#snippet trailing()}
          <span class="ink-pin-count ink-mono">{pinCount}</span>
        {/snippet}
      </MenuRow>
      <button type="button" class="ink-pin-release ink-focus" onclick={() => runAction('pins.closeAll')}>모두 닫기</button>
    </div>
  {/if}

  <p class="ink-tray-hd">빠른 동작</p>
  <MenuRow
    icon="restore"
    label="직전 판서 열기"
    combo={comboOf('restore')}
    {platform}
    keyDim={!hotkeysEnabled}
    onclick={() => runAction('draw.restore')}
  />
  <MenuRow
    icon="folder"
    label="저장 폴더 열기"
    onclick={() => runAction('folder.open')}
  />

  {#snippet footer()}
    <div class="ink-hotkey-row">
      <span class="ink-hotkey-label">단축키</span>
      <Keycap combo={comboOf('pause')} {platform} variant={hotkeysEnabled ? 'default' : 'dim'} />
      <Switch
        checked={hotkeysEnabled}
        label="단축키 켜기·끄기"
        onCheckedChange={(v) => client.setHotkeysEnabled(v)}
      />
    </div>
    <div class="ink-foot-split">
      <MenuRow
        label="설정…"
        combo={settingsCombo}
        {platform}
        onclick={() => runAction('settings.open')}
        class="ink-foot-settings"
      />
      <MenuRow
        label="끝내기"
        muted={true}
        onclick={() => runAction('app.quit')}
        class="ink-foot-quit"
      />
    </div>
  {/snippet}
</TrayPopover>

<style>
  .ink-tray-brand {
    flex: 1;
    font-size: 13.5px;
    font-weight: 600;
    color: var(--ink-text);
    white-space: nowrap;
  }

  .ink-tray-status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--ink-sub);
    white-space: nowrap;
  }

  .ink-tray-status-btn {
    border: 0;
    background: transparent;
    padding: 2px 5px;
    margin: -2px -5px;
    border-radius: var(--ink-r-key);
    font-family: inherit;
    color: var(--ink-accent-text);
    cursor: pointer;
  }

  .ink-tray-status-btn:hover {
    background: var(--ink-hover);
  }

  .ink-tray-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex: none;
  }

  .ink-tray-dot[data-tone='ok'] {
    background: var(--ink-ok);
  }

  .ink-tray-dot[data-tone='attn'] {
    background: var(--ink-attn);
  }

  .ink-tray-dot[data-tone='danger'] {
    background: var(--ink-danger);
    animation: ink-pulse-rec 1.6s ease-in-out infinite;
  }

  .ink-tray-dot[data-tone='off'] {
    background: transparent;
    box-shadow: inset 0 0 0 1.5px var(--ink-sub);
  }

  .ink-tray-hd {
    margin: 7px 10px 3px;
    font-size: 10.5px;
    font-weight: 500;
    letter-spacing: 0.04em;
    color: var(--ink-sub);
    white-space: nowrap;
  }

  .ink-tray-hd:first-child {
    margin-top: 3px;
  }

  .ink-pin-row {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .ink-pin-count {
    font-size: 10.5px;
    color: var(--ink-accent-text);
    background: var(--ink-accent-soft);
    padding: 0 5px;
    height: 17px;
    border-radius: 9px;
    display: inline-flex;
    align-items: center;
  }

  .ink-pin-release {
    border: 0;
    height: 22px;
    padding: 0 7px;
    margin-right: 4px;
    border-radius: var(--ink-r-key);
    background: var(--ink-chip-bg);
    color: var(--ink-text-2);
    font-family: inherit;
    font-size: 11px;
    white-space: nowrap;
    cursor: pointer;
    flex: none;
  }

  .ink-pin-release:hover {
    background: var(--ink-press);
    color: var(--ink-text);
  }

  .ink-hotkey-row {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 29px;
    padding: 0 10px;
    font-size: 13px;
    color: var(--ink-text);
  }

  .ink-hotkey-label {
    flex: 1;
    white-space: nowrap;
  }

  .ink-foot-split {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .ink-foot-split :global(.ink-foot-settings) {
    flex: 1;
  }

  .ink-foot-split :global(.ink-foot-quit) {
    width: auto;
    flex: none;
  }
</style>
