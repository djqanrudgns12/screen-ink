<script>
  import Icon from '../icons/Icon.svelte';
  import Keycap from '../components/Keycap.svelte';
  import { buildCheatSheetColumns } from '../models/cheatSheet.js';
  import { locale, syncLang, t } from '../i18n/index.svelte.js';
  import { exportShortcutSheetPdf } from '../pdf/download.js';

  /**
   * S16 단축키 한 장(F17) — state.shortcuts의 현재 값으로 3열 구성.
   * 아무 키나 누르면 닫히고, [PDF로 받기]·[×] 단추를 제공한다.
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
  let exportingPdf = $state(false);

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

  let platform = $derived(state?.platform ?? 'windows');
  let columns = $derived(
    buildCheatSheetColumns(state?.shortcuts ?? [], { platform, mode, lang: locale.lang }),
  );

  function closeSheet() {
    open = false;
    onClose?.();
  }

  async function handleExportPdf(e) {
    e?.stopPropagation?.();
    if (exportingPdf) return;
    exportingPdf = true;
    try {
      await exportShortcutSheetPdf(client, {
        shortcuts: state?.shortcuts ?? [],
        lang: locale.lang,
        os: platform,
        version: state?.version,
      });
    } finally {
      exportingPdf = false;
    }
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
    <div role="dialog" aria-modal="true" aria-label={t('shortcuts.cheat.aria')} class="ink-cheat-card">
      <header class="ink-cheat-head">
        <span class="ink-cheat-head-icon">
          <Icon name="shortcuts" size={20} />
        </span>
        <h1 class="ink-cheat-title">{t('shortcuts.cheat.title')}</h1>
        <span class="ink-cheat-hint">{t('shortcuts.cheat.hint')}</span>
        <button
          type="button"
          class="ink-cheat-pdf-btn ink-focus"
          data-cheat-ignore
          disabled={exportingPdf}
          onclick={handleExportPdf}
        >
          <Icon name="save" size={14} />
          <span>{exportingPdf ? t('pdf.action.generating') : t('pdf.action.download')}</span>
        </button>
        <button
          type="button"
          class="ink-cheat-close ink-focus"
          data-cheat-ignore
          aria-label={t('shortcuts.cheat.close')}
          onclick={closeSheet}
        >
          <Icon name="close" size={16} />
        </button>
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
                {#if row.keyGroups && row.keyGroups.length > 0}
                  <span class="ink-cheat-groups">
                    {#each row.keyGroups as grp, gi (gi)}
                      {#if gi > 0}
                        <span class="ink-cheat-sep">{row.separator}</span>
                      {/if}
                      <Keycap keys={grp} variant="island" />
                    {/each}
                  </span>
                {:else if row.keys && row.keys.length > 0}
                  <span class="ink-cheat-groups">
                    {#if row.keyPrefix}
                      <span class="ink-cheat-prefix ink-mono">{row.keyPrefix}</span>
                    {/if}
                    <Keycap keys={row.keys} variant="island" />
                  </span>
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
    width: min(920px, calc(100vw - 32px));
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
    font-family: var(--ink-font-display);
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

  .ink-cheat-pdf-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    padding: 0 10px;
    border: 1px solid var(--ink-line-strong);
    border-radius: var(--ink-r-btn);
    background: var(--ink-raised);
    color: var(--ink-text);
    font-family: inherit;
    font-size: 11.5px;
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
  }

  .ink-cheat-pdf-btn:hover:not(:disabled) {
    background: var(--ink-hover);
  }

  .ink-cheat-pdf-btn:disabled {
    opacity: 0.6;
    cursor: wait;
  }

  .ink-cheat-close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border: 0;
    border-radius: var(--ink-close-r);
    background: var(--ink-close-bg);
    color: var(--ink-sub);
    cursor: pointer;
    flex: none;
  }

  .ink-cheat-close:hover {
    color: var(--ink-text);
    background: var(--ink-hover);
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
    min-height: 26px;
    padding: 2px 6px;
    font-size: 12px;
    color: var(--ink-text);
  }

  .ink-cheat-label {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ink-cheat-groups {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex: none;
  }

  .ink-cheat-sep,
  .ink-cheat-prefix {
    font-size: 10.5px;
    color: var(--ink-sub);
    white-space: nowrap;
  }

  .ink-cheat-note {
    font-size: 11px;
    color: var(--ink-sub);
    white-space: nowrap;
    flex: none;
  }
</style>
