<script>
  import Icon from '../icons/Icon.svelte';
  import Keycap from './Keycap.svelte';
  import { t } from '../i18n/index.svelte.js';
  import {
    SHORTCUT_PRESETS,
    applyPreset,
    detectActivePreset,
    previewPresetWithClient,
    undoPresetApply,
  } from '../shortcuts/presets.js';

  const SLOT_I18N_KEYS = {
    region: 'shortcuts.slot.region',
    window: 'shortcuts.slot.window',
    record: 'shortcuts.slot.record',
    draw: 'shortcuts.slot.draw',
    zoom: 'shortcuts.slot.zoom',
    board: 'shortcuts.slot.board',
    last: 'shortcuts.slot.last',
    full: 'shortcuts.slot.full',
    active: 'shortcuts.slot.active',
    delay: 'shortcuts.slot.delay',
    restore: 'shortcuts.slot.restore',
    pause: 'shortcuts.slot.pause',
    pin: 'shortcuts.slot.pin',
    pinToggle: 'shortcuts.slot.pinToggle',
  };

  /**
   * 단축키 추천 묶음 선택기(R2-KEY-6):
   * [발표용 기본] · [Snipaste 습관] · [한 손으로(왼손)] 선택 → 바뀔 키 미리보기(겹침 표시) → [적용] → [되돌리기].
   * @type {{
   *   client: import('@screen-ink/client').InkClient,
   *   shortcuts: ReadonlyArray<{ id: string, label?: string, combo: string }>,
   *   platform?: 'windows' | 'macos',
   *   compact?: boolean,
   *   slotLabel?: (row: { id: string, label?: string }) => string,
   * }}
   */
  let {
    client,
    shortcuts = [],
    platform = 'windows',
    compact = false,
    slotLabel = undefined,
  } = $props();

  let selectedId = $state(/** @type {string | null} */ (null));
  let previewData = $state(/** @type {Awaited<ReturnType<typeof previewPresetWithClient>> | null} */ (null));
  let loadingPreview = $state(false);
  let applying = $state(false);
  let showAllPreview = $state(false);
  let lastApplied = $state(
    /** @type {{ undoSteps: number, failedCount: number, previous: Array<{ id: string, combo: string }> } | null} */ (null),
  );
  let undoIncomplete = $state(false);

  let activePresetId = $derived(detectActivePreset(shortcuts, platform));
  let previewLimit = $derived(compact ? 4 : 6);

  function resolveSlotLabel(item) {
    if (slotLabel) return slotLabel(item);
    const key = SLOT_I18N_KEYS[item.id];
    return key ? t(key) : item.label || item.id;
  }

  async function selectPreset(presetId) {
    if (applying) return;
    if (selectedId === presetId) {
      selectedId = null;
      previewData = null;
      showAllPreview = false;
      return;
    }
    selectedId = presetId;
    showAllPreview = false;
    loadingPreview = true;
    try {
      const res = await previewPresetWithClient(client, presetId, shortcuts, platform);
      if (selectedId === presetId) {
        previewData = res;
      }
    } finally {
      if (selectedId === presetId) loadingPreview = false;
    }
  }

  function closePreview() {
    selectedId = null;
    previewData = null;
    showAllPreview = false;
  }

  async function handleApply() {
    if (!selectedId || applying) return;
    const targetId = selectedId;
    applying = true;
    try {
      const res = await applyPreset(client, targetId, shortcuts, platform);
      undoIncomplete = false;
      lastApplied = { undoSteps: res.undoSteps, failedCount: res.failed.length, previous: res.previous };
      closePreview();
    } finally {
      applying = false;
    }
  }

  async function handleUndo() {
    if (!lastApplied || applying) return;
    const steps = lastApplied.undoSteps;
    applying = true;
    try {
      undoIncomplete = !(await undoPresetApply(client, steps, lastApplied.previous));
      lastApplied = null;
    } finally {
      applying = false;
    }
  }
</script>

<section class="ink-presets" class:is-compact={compact} aria-label={t('page.shortcuts.preset.aria')}>
  <div class="ink-presets-head">
    <div class="ink-presets-titles">
      <span class="ink-presets-title">{t('page.shortcuts.preset.title')}</span>
      {#if !compact}
        <span class="ink-presets-sub">{t('page.shortcuts.preset.sub')}</span>
      {/if}
    </div>
  </div>

  <div class="ink-presets-grid" role="group" aria-label={t('page.shortcuts.preset.aria')}>
    {#each SHORTCUT_PRESETS as preset (preset.id)}
      {@const isCurrent = activePresetId === preset.id}
      {@const isSelected = selectedId === preset.id}
      <button
        type="button"
        class="ink-preset-card ink-focus"
        data-preset={preset.id}
        data-current={isCurrent ? 'true' : undefined}
        data-selected={isSelected ? 'true' : undefined}
        aria-pressed={isSelected}
        onclick={() => selectPreset(preset.id)}
      >
        <span class="ink-preset-top">
          <span class="ink-preset-name">{t(preset.titleKey)}</span>
          {#if isCurrent}
            <span class="ink-preset-badge" data-tone="ok">{t('page.shortcuts.preset.current')}</span>
          {:else if preset.badgeKey && !compact}
            <span class="ink-preset-badge">{t(preset.badgeKey)}</span>
          {/if}
        </span>
        {#if !compact}
          <span class="ink-preset-desc">{t(preset.descKey)}</span>
        {/if}
      </button>
    {/each}
  </div>

  {#if selectedId && previewData}
    {@const visibleChanges = showAllPreview
      ? previewData.changes
      : previewData.changes.slice(0, previewLimit)}
    {@const hiddenCount = Math.max(0, previewData.changes.length - visibleChanges.length)}
    <div class="ink-preset-preview" role="region" aria-label={t('page.shortcuts.preset.previewTitle')}>
      <div class="ink-preset-preview-head">
        <span class="ink-preset-preview-title">
          {t('page.shortcuts.preset.previewTitle')}
          {#if previewData.changes.length > 0}
            <span class="ink-preset-preview-count ink-mono">({previewData.changes.length})</span>
          {/if}
        </span>
        {#if previewData.conflictCount > 0}
          <span class="ink-preset-conflict-pill">
            <Icon name="warning-triangle" size={12} />
            <span>{t('page.shortcuts.preset.conflictSummary', { n: previewData.conflictCount })}</span>
          </span>
        {/if}
      </div>

      {#if loadingPreview}
        <p class="ink-preset-same">{t('page.shortcuts.preset.loading')}</p>
      {:else if previewData.changes.length === 0}
        <p class="ink-preset-same">{t('page.shortcuts.preset.sameAsCurrent')}</p>
      {:else}
        <ul class="ink-preset-diff-list">
          {#each visibleChanges as item (item.id)}
            <li class="ink-preset-diff-row" data-status={item.status}>
              <span class="ink-preset-diff-main">
                <span class="ink-preset-diff-label">{resolveSlotLabel(item)}</span>
                {#if item.status === 'conflict'}
                  <span class="ink-preset-tag" data-kind="conflict">
                    {item.owner || t('page.shortcuts.preset.conflictTag')}
                  </span>
                {:else if item.status === 'warn'}
                  <span class="ink-preset-tag" data-kind="warn">
                    {t('page.shortcuts.preset.warnTag')}
                  </span>
                {/if}
              </span>
              <span class="ink-preset-diff-keys">
                {#if item.before}
                  <Keycap combo={item.before} {platform} variant="dim" />
                {:else}
                  <span class="ink-preset-empty">{t('page.shortcuts.preset.emptyKey')}</span>
                {/if}
                <span class="ink-preset-arrow" aria-hidden="true">→</span>
                <Keycap combo={item.after} {platform} />
              </span>
            </li>
            {#if item.reason && (item.status === 'conflict' || item.status === 'warn')}
              <li class="ink-preset-diff-reason">{item.reason}</li>
            {/if}
          {/each}
        </ul>
        {#if hiddenCount > 0}
          <button
            type="button"
            class="ink-preset-more ink-focus"
            onclick={() => (showAllPreview = true)}
          >
            {t('page.shortcuts.preset.moreCount', { n: hiddenCount })}
          </button>
        {/if}
      {/if}

      <div class="ink-preset-actions">
        <button
          type="button"
          class="ink-preset-btn ink-preset-btn-sec ink-focus"
          disabled={applying}
          onclick={closePreview}
        >
          {t('common.cancel')}
        </button>
        {#if previewData.changes.length > 0}
          <button
            type="button"
            class="ink-preset-btn ink-preset-btn-pri ink-focus"
            disabled={applying}
            onclick={handleApply}
          >
            {applying ? t('page.shortcuts.preset.applying') : t('page.shortcuts.preset.apply')}
          </button>
        {/if}
      </div>
    </div>
  {:else if undoIncomplete}
    <div class="ink-preset-applied" role="status">{t('page.shortcuts.preset.undoIncomplete')}</div>
  {:else if lastApplied}
    <div class="ink-preset-applied" role="status">
      <span class="ink-preset-applied-text">
        <Icon name="check-circle" size={14} />
        <span>
          {lastApplied.failedCount > 0
            ? t('page.shortcuts.preset.appliedPartial', { n: lastApplied.failedCount })
            : t('page.shortcuts.preset.applied')}
        </span>
      </span>
      {#if lastApplied.undoSteps > 0}
        <button
          type="button"
          class="ink-preset-btn ink-preset-btn-sec ink-focus"
          disabled={applying}
          onclick={handleUndo}
        >
          {t('page.shortcuts.preset.undo')}
        </button>
      {/if}
    </div>
  {/if}
</section>

<style>
  .ink-presets {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .ink-presets-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
  }

  .ink-presets-titles {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
  }

  .ink-presets-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-text-2);
  }

  .ink-presets-sub {
    font-size: 11.5px;
    color: var(--ink-sub);
  }

  .ink-presets-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }

  .ink-preset-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 9px 11px;
    border: 0;
    border-radius: var(--ink-r-group);
    background: var(--ink-group-bg);
    box-shadow: var(--ink-group-ring);
    color: var(--ink-text);
    font-family: inherit;
    text-align: left;
    cursor: pointer;
    transition: background-color 110ms ease, box-shadow 110ms ease;
  }

  .ink-presets.is-compact .ink-preset-card {
    padding: 7px 9px;
  }

  .ink-preset-card:hover {
    background: var(--ink-hover);
  }

  .ink-preset-card[data-selected='true'] {
    background: var(--ink-rec-bg);
    box-shadow: inset 0 0 0 1.5px var(--ink-accent-icon);
  }

  .ink-preset-top {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }

  .ink-preset-name {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--ink-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ink-presets.is-compact .ink-preset-name {
    font-size: 12px;
  }

  .ink-preset-badge {
    flex: none;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--ink-chip-bg);
    color: var(--ink-sub);
    font-size: 10.5px;
    font-weight: 600;
    white-space: nowrap;
  }

  .ink-preset-badge[data-tone='ok'] {
    background: color-mix(in srgb, var(--ink-ok) 15%, transparent);
    color: var(--ink-ok);
  }

  .ink-preset-desc {
    font-size: 11px;
    line-height: 1.35;
    color: var(--ink-sub);
  }

  .ink-preset-preview {
    padding: 10px 12px;
    border-radius: var(--ink-r-group);
    background: var(--ink-group-bg);
    box-shadow: var(--ink-group-ring);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .ink-preset-preview-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .ink-preset-preview-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-text);
  }

  .ink-preset-preview-count {
    font-weight: 500;
    color: var(--ink-sub);
  }

  .ink-preset-conflict-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 7px;
    border-radius: 999px;
    background: var(--ink-attn-bg);
    color: var(--ink-accent-text);
    font-size: 11px;
    font-weight: 600;
  }

  .ink-preset-same {
    margin: 0;
    font-size: 12px;
    color: var(--ink-sub);
  }

  .ink-preset-diff-list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .ink-preset-diff-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    min-height: 24px;
    font-size: 12px;
  }

  .ink-preset-diff-main {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .ink-preset-diff-label {
    color: var(--ink-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ink-preset-tag {
    flex: none;
    padding: 1px 6px;
    border-radius: 999px;
    font-size: 10.5px;
    font-weight: 600;
    background: var(--ink-attn-bg);
    color: var(--ink-accent-text);
  }

  .ink-preset-tag[data-kind='conflict'] {
    background: color-mix(in srgb, var(--ink-danger) 14%, transparent);
    color: var(--ink-danger-text);
  }

  .ink-preset-diff-reason {
    margin: -2px 0 2px;
    font-size: 11px;
    color: var(--ink-sub);
  }

  .ink-preset-diff-keys {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    flex: none;
  }

  .ink-preset-arrow {
    font-size: 11px;
    color: var(--ink-sub);
  }

  .ink-preset-empty {
    font-size: 11px;
    color: var(--ink-sub);
  }

  .ink-preset-more {
    align-self: flex-start;
    padding: 2px 0;
    border: 0;
    background: transparent;
    color: var(--ink-accent-text);
    font-family: inherit;
    font-size: 11.5px;
    font-weight: 600;
    cursor: pointer;
  }

  .ink-preset-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }

  .ink-preset-applied {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 12px;
    border-radius: var(--ink-r-group);
    background: color-mix(in srgb, var(--ink-ok) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink-ok) 32%, transparent);
  }

  .ink-preset-applied-text {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-text);
  }

  .ink-preset-applied-text :global(svg) {
    color: var(--ink-ok);
  }

  .ink-preset-btn {
    height: 26px;
    padding: 0 11px;
    border: 0;
    border-radius: var(--ink-r-btn);
    font-family: inherit;
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
  }

  .ink-preset-btn-sec {
    background: var(--ink-btn-sec-bg);
    box-shadow: var(--ink-btn-sec-shadow);
    color: var(--ink-text);
  }

  .ink-preset-btn-sec:hover:not(:disabled) {
    background: var(--ink-hover);
  }

  .ink-preset-btn-pri {
    background: var(--ink-invert);
    color: var(--ink-on-invert);
    font-weight: 600;
  }

  .ink-preset-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
