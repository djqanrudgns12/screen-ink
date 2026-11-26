<script>
  import AppIcon from '../icons/AppIcon.svelte';
  import Icon from '../icons/Icon.svelte';
  import TrayPopover from '../components/TrayPopover.svelte';
  import MenuRow from '../components/MenuRow.svelte';
  import Switch from '../components/Switch.svelte';
  import Keycap from '../components/Keycap.svelte';
  import { syncLang, t } from '../i18n/index.svelte.js';
  import { TRAY_TILES, formatElapsed, resolveTrayStatus } from '../models/trayMenu.js';

  /**
   * S01 메뉴 막대 / 작업 표시줄 팝오버 화면(F03 · R2-UI-TRAY · 후보 #2 Paperlogy).
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
      if (!alive) return;
      state = s;
      syncLang(s);
    });
    const off = client.subscribe((e) => {
      if (e.type === 'state') {
        state = e.state;
        syncLang(e.state);
      }
    });
    return () => {
      alive = false;
      off();
    };
  });

  function comboOf(id) {
    return state?.shortcuts?.find((s) => s.id === id)?.combo ?? '';
  }

  let platform = $derived(state?.platform ?? 'windows');
  let hotkeysEnabled = $derived(state?.hotkeysEnabled ?? true);
  let recording = $derived(Boolean(state?.recording?.active));
  let pinnedCount = $derived(state?.pinnedCount ?? 0);
  let pinCount = $derived(state?.pins ?? 0);
  let status = $derived(resolveTrayStatus(state));
  let settingsCombo = $derived(platform === 'macos' ? 'Cmd+,' : 'Ctrl+,');

  /** @param {string} action */
  function runAction(action) {
    client.run(/** @type {any} */ (action));
  }
</script>

<TrayPopover label={t('tray.menu.aria')} {platform} {onClose} class={className}>
  {#snippet header()}
    <AppIcon size={20} />
    <span class="ink-tray-brand">{t('tray.brand')}</span>
    {#if status.kind === 'recording'}
      <span class="ink-tray-status" data-tone="danger">
        <i class="ink-tray-dot" data-tone="danger"></i>
        <span>
          {t('tray.status.recording')}
          <span class="ink-mono">{formatElapsed(status.elapsedMs)}</span>
        </span>
      </span>
    {:else if status.kind === 'permission'}
      <button
        type="button"
        class="ink-tray-status ink-tray-status-btn ink-focus"
        data-tone="attn"
        onclick={() => runAction(status.action ?? 'firstRun.open')}
      >
        <i class="ink-tray-dot" data-tone="attn"></i>
        <span>{t('tray.status.permission')}</span>
      </button>
    {:else if status.kind === 'paused'}
      <span class="ink-tray-status" data-tone="off">
        <i class="ink-tray-dot" data-tone="off"></i>
        <span>{t('tray.status.paused')}</span>
      </span>
    {:else if status.kind === 'conflict'}
      <button
        type="button"
        class="ink-tray-status ink-tray-status-btn ink-focus"
        data-tone="attn"
        onclick={() => runAction(status.action ?? 'settings.open')}
      >
        <i class="ink-tray-dot" data-tone="attn"></i>
        <span>{t('tray.status.conflict', { n: status.count ?? 0 })}</span>
      </button>
    {:else}
      <span class="ink-tray-status" data-tone="ok">
        <i class="ink-tray-dot" data-tone="ok"></i>
        <span>{t('tray.status.ready')}</span>
      </span>
    {/if}
  {/snippet}

  <div class="ink-tray-tiles" role="group" aria-label={t('tray.menu.aria')}>
    {#each TRAY_TILES as tile (tile.id)}
      {@const isRec = tile.id === 'record' && recording}
      {@const tileLabel = isRec ? t('tray.tile.recordStop') : t(tile.labelKey)}
      {@const combo = comboOf(tile.slotId)}
      <button
        type="button"
        role="menuitem"
        class="ink-tray-tile"
        data-recording={isRec ? 'true' : undefined}
        data-menu-label={tileLabel}
        onclick={() => runAction(tile.action)}
      >
        <span class="ink-tray-tile-top">
          <span class="ink-tray-tile-icon">
            <Icon name={tile.icon} size={18} accent={true} />
          </span>
          {#if combo}
            <Keycap
              {combo}
              {platform}
              variant={!hotkeysEnabled ? 'dim' : 'default'}
              class="ink-tile-keycap"
            />
          {/if}
        </span>
        <span class="ink-tray-tile-label">{tileLabel}</span>
      </button>
    {/each}
  </div>

  <div class="ink-tray-sep-inline" role="separator"></div>

  <MenuRow
    icon="draw"
    label={t('tray.item.draw')}
    combo={comboOf('draw')}
    {platform}
    keyDim={!hotkeysEnabled}
    onclick={() => runAction('draw')}
  />
  <MenuRow
    icon="board"
    label={t('tray.item.board')}
    combo={comboOf('board')}
    {platform}
    keyDim={!hotkeysEnabled}
    onclick={() => runAction('board')}
  />
  <MenuRow
    icon="zoom"
    label={t('tray.item.zoom')}
    combo={comboOf('zoom')}
    {platform}
    keyDim={!hotkeysEnabled}
    onclick={() => runAction('zoom')}
  />

  {#if platform !== 'macos'}
    <div class="ink-pin-row">
      <MenuRow
        icon="window-top"
        label={t('tray.item.topmost')}
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
          {t('tray.action.releaseAll')}
        </button>
      {/if}
    </div>
  {/if}

  <div class="ink-tray-sep-inline" role="separator"></div>

  <MenuRow
    icon="restore"
    label={t('tray.item.reopenLast')}
    combo={comboOf('restore')}
    {platform}
    keyDim={!hotkeysEnabled}
    onclick={() => runAction('draw.restore')}
  />
  <MenuRow
    icon="folder"
    label={t('tray.item.openFolder')}
    {platform}
    onclick={() => runAction('folder.open')}
  />

  {#if pinCount > 0}
    <div class="ink-pin-row">
      <MenuRow
        icon="pin"
        label={t('tray.item.pinsCount', { n: pinCount })}
        combo=""
        {platform}
        onclick={() => runAction('pins.closeAll')}
      />
      <button
        type="button"
        class="ink-pin-release ink-focus"
        onclick={() => runAction('pins.closeAll')}
      >
        {t('tray.action.closeAll')}
      </button>
    </div>
  {/if}

  {#snippet footer()}
    <div class="ink-hotkey-row">
      <span class="ink-hotkey-label">{t('tray.footer.shortcuts')}</span>
      <Keycap combo={comboOf('pause')} {platform} variant={hotkeysEnabled ? 'default' : 'dim'} class="ink-foot-keycap" />
      <Switch
        checked={hotkeysEnabled}
        label={t('tray.footer.shortcutsToggle')}
        onCheckedChange={(v) => client.setHotkeysEnabled(v)}
      />
    </div>
    <div class="ink-foot-split">
      <MenuRow
        label={t('tray.footer.settings')}
        combo={settingsCombo}
        {platform}
        onclick={() => runAction('settings.open')}
        class="ink-foot-settings"
      />
      <MenuRow
        label={t('tray.footer.quit')}
        muted={true}
        {platform}
        onclick={() => runAction('app.quit')}
        class="ink-foot-quit"
      />
    </div>
  {/snippet}
</TrayPopover>

<style>
  .ink-tray-brand {
    flex: 1;
    min-width: 0;
    font-size: 13.5px;
    font-weight: 600;
    line-height: 1.2;
    color: var(--ink-text);
    white-space: nowrap;
  }

  .ink-tray-status {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: var(--ink-text-xs);
    font-weight: 500;
    line-height: 1.2;
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

  /* 상단 2×2 동작 타일 4칸 */
  .ink-tray-tiles {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px;
    padding: 2px 2px 4px;
  }

  .ink-tray-tile {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 6px;
    min-height: 54px;
    padding: 8px 9px;
    box-sizing: border-box;
    border: 1px solid var(--ink-line-soft);
    border-radius: 9px;
    background: var(--ink-surface);
    color: var(--ink-text);
    font-family: inherit;
    text-align: left;
    cursor: pointer;
    outline: none;
    transition: background-color 120ms ease-out, border-color 120ms ease-out, color 120ms ease-out;
  }

  .ink-tray-tile:hover {
    background: var(--ink-hover);
    border-color: var(--ink-line-strong);
  }

  .ink-tray-tile:active {
    background: var(--ink-press);
  }

  .ink-tray-tile:focus-visible {
    background: var(--ink-invert);
    border-color: var(--ink-invert);
    color: var(--ink-on-invert);
  }

  .ink-tray-tile:focus-visible .ink-tray-tile-icon {
    color: var(--ink-on-invert);
  }

  .ink-tray-tile:focus-visible :global(.ink-kbd) {
    color: var(--ink-key-on-invert-text);
    background: var(--ink-key-on-invert-bg);
    border-color: var(--ink-key-on-invert-border);
  }

  .ink-tray-tile[data-recording='true'] {
    background: rgba(229, 72, 77, 0.12);
    border-color: rgba(229, 72, 77, 0.45);
    color: var(--ink-danger-text);
  }

  .ink-tray-tile-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 4px;
    width: 100%;
  }

  .ink-tray-tile-icon {
    display: inline-flex;
    align-items: center;
    color: var(--ink-text-2);
    flex: none;
  }

  .ink-tray-tile :global(.ink-tile-keycap .ink-kbd[data-variant='default']),
  .ink-hotkey-row :global(.ink-foot-keycap .ink-kbd[data-variant='default']) {
    color: var(--ink-sub);
    font-weight: 400;
  }

  .ink-tray-tile-label {
    font-size: var(--ink-text-base);
    font-weight: 600;
    line-height: 1.2;
    letter-spacing: var(--ink-tracking-ko);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ink-tray-sep-inline {
    height: 1px;
    background: var(--ink-line-soft);
    margin: 4px 6px;
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
    font-size: var(--ink-text-xs);
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
    gap: 6px;
    height: 28px;
    padding: 0 8px;
    font-size: var(--ink-text-base);
    font-weight: 500;
    line-height: 1.2;
    color: var(--ink-text);
  }

  .ink-hotkey-label {
    flex: 1;
    min-width: 0;
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
