<script>
  import { Dialog } from 'bits-ui';
  import AppIcon from '../icons/AppIcon.svelte';
  import Icon from '../icons/Icon.svelte';
  import Keycap from '../components/Keycap.svelte';
  import { formatFirstRunConflict } from '../models/settingsMeta.js';

  /**
   * S06 첫 실행 시트(F08).
   * [시작하기] · Esc · 바깥 클릭 = 닫기, [단축키 바꾸기] = 닫고 단축키 탭.
   * @type {{
   *   client: import('@screen-ink/client').InkClient,
   *   open?: boolean,
   *   onClose?: () => void,
   *   onChangeShortcuts?: () => void,
   * }}
   */
  let {
    client,
    open = $bindable(true),
    onClose = undefined,
    onChangeShortcuts = undefined,
  } = $props();

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

  let platform = $derived(state?.platform ?? 'windows');
  let shortcuts = $derived(state?.shortcuts ?? []);
  let conflictInfo = $derived(formatFirstRunConflict(state?.conflicts, platform));

  const CORE_ITEMS = [
    { id: 'region', icon: 'region', label: '범위 캡처', hint: '알아서 찾아요', fallbackWin: 'F1', fallbackMac: 'F1' },
    { id: 'draw', icon: 'draw', label: '판서', hint: '멈추고 쓰기', fallbackWin: 'Alt+1', fallbackMac: 'Option+1' },
    { id: 'zoom', icon: 'zoom', label: '실시간 확대', hint: '움직이는 그대로', fallbackWin: 'Alt+2', fallbackMac: 'Option+2' },
  ];

  function comboFor(item) {
    const row = shortcuts.find((s) => s.id === item.id);
    if (row) return row.combo;
    return platform === 'macos' ? item.fallbackMac : item.fallbackWin;
  }

  function handleOpenChange(next) {
    open = next;
    if (!next) onClose?.();
  }

  function handleStart() {
    open = false;
    onClose?.();
  }

  function handleChangeShortcuts() {
    open = false;
    if (onChangeShortcuts) {
      onChangeShortcuts();
    } else {
      client.run('settings.open');
      onClose?.();
    }
  }
</script>

<Dialog.Root {open} onOpenChange={handleOpenChange}>
  <Dialog.Portal>
    <Dialog.Overlay class="ink-first-overlay" />
    <Dialog.Content class="ink-first-dialog" aria-label="첫 실행 안내">
      <div class="ink-first-head">
        <AppIcon size={56} label="screen-ink" />
        <div class="ink-first-titles">
          <Dialog.Title class="ink-first-title">screen-ink가 준비됐어요</Dialog.Title>
          <Dialog.Description class="ink-first-sub">
            트레이에 있다가 단축키를 누르면 나타나요
          </Dialog.Description>
        </div>
      </div>

      <section class="ink-first-section">
        <p class="ink-first-sec-label">먼저 이 셋만</p>
        <div class="ink-first-group">
          {#each CORE_ITEMS as item (item.id)}
            {@const combo = comboFor(item)}
            <div class="ink-first-row">
              <span class="ink-first-icon">
                <Icon name={item.icon} size={20} />
              </span>
              <span class="ink-first-row-label">
                {item.label}
                <small class="ink-first-row-hint">{item.hint}</small>
              </span>
              {#if combo}
                <Keycap {combo} {platform} />
              {:else}
                <span class="ink-first-empty">비움</span>
              {/if}
            </div>
          {/each}
        </div>
      </section>

      {#if conflictInfo}
        <div role="status" class="ink-first-notice" data-kind="conflict">
          <span class="ink-first-notice-icon" data-kind="conflict">
            <Icon name="warning" size={16} />
          </span>
          <p class="ink-first-notice-text">
            {conflictInfo.prefix}{#if conflictInfo.app}<strong>{conflictInfo.app}</strong>{/if}{conflictInfo.suffix}
          </p>
          {#if conflictInfo.canHowto}
            <button
              type="button"
              class="ink-first-howto ink-focus"
              onclick={() => client.run('snipaste.howto')}
            >
              끄는 법
            </button>
          {/if}
        </div>
      {:else}
        <div role="status" class="ink-first-notice" data-kind="info">
          <span class="ink-first-notice-icon" data-kind="info">
            <Icon name="info" size={16} />
          </span>
          <p class="ink-first-notice-text">
            F1·F2·F3을 쓰는 동안 다른 앱의 도움말·이름 바꾸기·찾기가 막혀요
          </p>
        </div>
      {/if}

      <div class="ink-first-footer">
        <p class="ink-first-help">
          <Keycap label="?" />
          <span>모드 안에서 언제든 단축키 한 장</span>
        </p>
        <div class="ink-first-actions">
          <button
            type="button"
            class="ink-first-btn ink-first-btn-sec ink-focus"
            onclick={handleChangeShortcuts}
          >
            단축키 바꾸기
          </button>
          <button
            type="button"
            class="ink-first-btn ink-first-btn-pri ink-focus"
            onclick={handleStart}
          >
            시작하기
          </button>
        </div>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.ink-first-overlay) {
    position: fixed;
    inset: 0;
    z-index: 70;
    background: var(--ink-sheet-scrim);
  }

  :global(.ink-first-dialog) {
    position: fixed;
    left: 50%;
    top: 50%;
    z-index: 71;
    transform: translate(-50%, -50%);
    width: min(520px, calc(100vw - 32px));
    box-sizing: border-box;
    padding: 28px 28px 22px;
    border-radius: var(--ink-r-modal);
    background: var(--ink-surface);
    color: var(--ink-text);
    box-shadow: var(--ink-shadow-modal);
    font-family: var(--ink-font-sans);
    display: flex;
    flex-direction: column;
    gap: 18px;
    outline: none;
  }

  .ink-first-head {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .ink-first-titles {
    min-width: 0;
  }

  :global(.ink-first-title) {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
    color: var(--ink-text);
  }

  :global(.ink-first-sub) {
    margin: 4px 0 0;
    font-size: 13px;
    color: var(--ink-sub);
  }

  .ink-first-section {
    display: flex;
    flex-direction: column;
  }

  .ink-first-sec-label {
    margin: 0 4px 7px;
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.04em;
    color: var(--ink-sub);
  }

  .ink-first-group {
    background: var(--ink-group-bg);
    border-radius: var(--ink-r-group);
    box-shadow: var(--ink-group-ring);
    overflow: hidden;
  }

  .ink-first-row {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 40px;
    padding: 0 14px;
    font-size: 13.5px;
    color: var(--ink-text);
    border-top: 1px solid var(--ink-hair);
  }

  .ink-first-row:first-child {
    border-top: 0;
  }

  .ink-first-icon {
    display: inline-flex;
    color: var(--ink-text-2);
    flex: none;
  }

  .ink-first-row-label {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
  }

  .ink-first-row-hint {
    margin-left: 6px;
    font-size: 12px;
    color: var(--ink-sub);
  }

  .ink-first-empty {
    font-size: 11px;
    color: var(--ink-sub);
    white-space: nowrap;
  }

  .ink-first-notice {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 10px 9px 14px;
    border-radius: var(--ink-r-group);
    background: var(--ink-group-bg);
    box-shadow: var(--ink-group-ring);
  }

  .ink-first-notice[data-kind='conflict'] {
    background: var(--ink-attn-bg);
    box-shadow: var(--ink-attn-ring);
  }

  .ink-first-notice-icon {
    display: inline-flex;
    flex: none;
    color: var(--ink-text-2);
  }

  .ink-first-notice-icon[data-kind='conflict'] {
    color: var(--ink-attn);
  }

  .ink-first-notice-text {
    margin: 0;
    flex: 1;
    min-width: 0;
    font-size: 12.5px;
    color: var(--ink-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ink-first-notice-text strong {
    font-weight: 600;
  }

  .ink-first-howto {
    height: 26px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--ink-r-btn);
    background: var(--ink-btn-sec-bg);
    box-shadow: var(--ink-btn-sec-shadow);
    color: var(--ink-text);
    font-family: inherit;
    font-size: 12px;
    white-space: nowrap;
    cursor: pointer;
    flex: none;
  }

  .ink-first-howto:hover {
    background: var(--ink-hover);
  }

  .ink-first-howto:active {
    opacity: 0.85;
  }

  .ink-first-footer {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .ink-first-help {
    margin: 0;
    flex: 1;
    min-width: 0;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    color: var(--ink-sub);
    white-space: nowrap;
  }

  .ink-first-actions {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    flex: none;
  }

  .ink-first-btn {
    height: 34px;
    border: 0;
    border-radius: var(--ink-r-btn);
    font-family: inherit;
    font-size: 13px;
    white-space: nowrap;
    cursor: pointer;
  }

  .ink-first-btn-sec {
    padding: 0 16px;
    background: var(--ink-btn-sec-bg);
    box-shadow: var(--ink-btn-sec-shadow);
    color: var(--ink-text);
  }

  .ink-first-btn-sec:hover {
    background: var(--ink-hover);
  }

  .ink-first-btn-pri {
    padding: 0 20px;
    background: var(--ink-invert);
    color: var(--ink-on-invert);
    font-weight: 600;
  }

  .ink-first-btn:active {
    opacity: 0.85;
  }
</style>
