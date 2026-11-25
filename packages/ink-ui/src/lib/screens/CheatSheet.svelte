<script>
  import Icon from '../icons/Icon.svelte';
  import Keycap from '../components/Keycap.svelte';
  import { buildCheatSheetColumns } from '../models/cheatSheet.js';

  /**
   * S16 단축키 한 장(F17) — state.shortcuts의 현재 값으로 3열 구성.
   * 아무 키나 누르면 닫힌다.
   * @type {{
   *   client: import('@screen-ink/client').InkClient,
   *   mode?: 'draw' | 'capture' | 'zoom',
   *   open?: boolean,
   *   onClose?: () => void,
   *   class?: string,
   * }}
   */
  let {
    client,
    mode = 'draw',
    open = $bindable(true),
    onClose = undefined,
    class: className = '',
  } = $props();

  const MODIFIER_ONLY = new Set(['Shift', 'Control', 'Alt', 'Meta', 'OS']);

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
  let columns = $derived(
    buildCheatSheetColumns(state?.shortcuts ?? [], { platform, mode }),
  );

  function closeSheet() {
    open = false;
    onClose?.();
  }

  function handleWindowKeydown(e) {
    if (!open) return;
    if (MODIFIER_ONLY.has(e.key)) return;
    if (e.target instanceof Element && e.target.closest('[data-cheat-ignore]')) return;
    e.preventDefault();
    closeSheet();
  }

  function handleBackdropClick(e) {
    if (e.target === e.currentTarget) {
      closeSheet();
    }
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#if open}
  <div
    class="ink-cheat-backdrop {className}"
    data-theme="dark"
    role="presentation"
    onclick={handleBackdropClick}
  >
    <div role="dialog" aria-modal="true" aria-label="단축키 한 장" class="ink-cheat-card">
      <header class="ink-cheat-head">
        <span class="ink-cheat-head-icon">
          <Icon name="shortcuts" size={20} />
        </span>
        <h1 class="ink-cheat-title">단축키 한 장</h1>
        <span class="ink-cheat-hint">아무 키나 누르면 닫혀요</span>
      </header>

      <div class="ink-cheat-grid">
        {#each columns as col (col.id)}
          <section class="ink-cheat-col" data-active={col.active ? 'true' : undefined}>
            <p class="ink-cheat-col-head" data-active={col.active ? 'true' : undefined}>
              {#if col.active}
                <span class="ink-cheat-dot" aria-hidden="true"></span>
              {/if}
              <span>{col.title}</span>
            </p>
            {#each col.rows as row (row.label)}
              <div class="ink-cheat-row">
                <span class="ink-cheat-label">{row.label}</span>
                {#if row.keys && row.keys.length > 0}
                  <Keycap keys={row.keys} variant="island" />
                {:else if row.note}
                  <span class="ink-cheat-note {row.noteMono ? 'ink-mono' : ''}">{row.note}</span>
                {/if}
              </div>
            {/each}
          </section>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .ink-cheat-backdrop {
    position: fixed;
    inset: 0;
    z-index: 65;
    display: grid;
    place-items: center;
    padding: 16px;
    background: var(--ink-scrim);
    font-family: var(--ink-font-sans);
  }

  .ink-cheat-card {
    width: min(820px, calc(100vw - 32px));
    box-sizing: border-box;
    padding: 18px 18px 16px;
    border-radius: var(--ink-r-modal);
    background: var(--ink-surface);
    color: var(--ink-text);
    box-shadow: var(--ink-shadow-modal);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .ink-cheat-head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 6px;
  }

  .ink-cheat-head-icon {
    display: inline-flex;
    color: var(--ink-text);
    flex: none;
  }

  .ink-cheat-title {
    margin: 0;
    flex: 1;
    font-size: 15px;
    font-weight: 600;
    color: var(--ink-text);
    white-space: nowrap;
  }

  .ink-cheat-hint {
    font-size: 11.5px;
    color: var(--ink-sub);
    white-space: nowrap;
  }

  .ink-cheat-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }

  .ink-cheat-col {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px;
    border-radius: var(--ink-r-group);
    min-width: 0;
  }

  .ink-cheat-col[data-active='true'] {
    background: var(--ink-raised);
    box-shadow: var(--ink-accent-ring);
  }

  .ink-cheat-col-head {
    margin: 0 6px 8px;
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.04em;
    color: var(--ink-sub);
    display: flex;
    align-items: center;
    gap: 8px;
    white-space: nowrap;
  }

  .ink-cheat-col-head[data-active='true'] {
    color: var(--ink-accent-text);
  }

  .ink-cheat-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ink-accent);
    flex: none;
  }

  .ink-cheat-row {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 26px;
    padding: 0 6px;
    font-size: 12.5px;
    color: var(--ink-text);
  }

  .ink-cheat-label {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ink-cheat-note {
    font-size: 11px;
    color: var(--ink-sub);
    white-space: nowrap;
    flex: none;
  }
</style>
