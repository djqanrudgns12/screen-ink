<script>
  import { locale, t } from '../i18n/index.svelte.js';
  import { formatComboReadable } from '../models/shortcutDisplay.js';

  /**
   * Group 안 한 줄(라벨 · 보조 · 컨트롤) + 단축키 입력 중/오류 상태 지원.
   * @type {{
   *   label: string,
   *   sub?: string,
   *   hoverSub?: string,
   *   subMono?: boolean,
   *   status?: '' | 'ok' | 'attn' | 'danger',
   *   later?: boolean,
   *   danger?: boolean,
   *   recording?: boolean,
   *   shaking?: boolean,
   *   clickable?: boolean,
   *   error?: string,
   *   warning?: string,
   *   note?: string,
   *   suggestions?: string[],
   *   platform?: 'windows' | 'macos',
   *   onSelectSuggestion?: (combo: string) => void,
   *   onclick?: () => void,
   *   children?: import('svelte').Snippet,
   *   class?: string,
   * }}
   */
  let {
    label,
    sub = '',
    hoverSub = '',
    subMono = false,
    status = '',
    later = false,
    danger = false,
    recording = false,
    shaking = false,
    clickable = false,
    error = '',
    warning = '',
    note = '',
    suggestions = [],
    platform = undefined,
    onSelectSuggestion = undefined,
    onclick = undefined,
    children = undefined,
    class: className = '',
  } = $props();

  let resolvedPlatform = $derived(platform ?? (locale.os === 'mac' ? 'macos' : 'windows'));

  function handleKeydown(e) {
    if (!clickable || !onclick) return;
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onclick();
    }
  }
</script>

<div
  class="ink-row-wrap {className}"
  data-recording={recording ? 'true' : undefined}
  data-shaking={shaking ? 'true' : undefined}
>
  {#if clickable}
    <div
      role="button"
      tabindex="0"
      class="ink-row ink-row-clickable ink-focus"
      onclick={onclick}
      onkeydown={handleKeydown}
    >
      <span class="ink-row-label" data-danger={danger ? 'true' : undefined}>
        {label}
        {#if later}
          <span class="ink-later-tag">{t('common.later')}</span>
        {/if}
      </span>
      {#if sub}
        <span class="ink-row-sub {subMono ? 'ink-mono' : ''}" data-recording={recording ? 'true' : undefined}>
          {sub}
        </span>
      {:else if hoverSub}
        <span class="ink-row-sub ink-row-sub-hover">
          {hoverSub}
        </span>
      {/if}
      {#if status}
        <span class="ink-status-dot" data-status={status} aria-hidden="true"></span>
      {/if}
      {@render children?.()}
    </div>
  {:else}
    <div class="ink-row">
      <span class="ink-row-label" data-danger={danger ? 'true' : undefined}>
        {label}
        {#if later}
          <span class="ink-later-tag">{t('common.later')}</span>
        {/if}
      </span>
      {#if sub}
        <span class="ink-row-sub {subMono ? 'ink-mono' : ''}" data-recording={recording ? 'true' : undefined}>
          {sub}
        </span>
      {:else if hoverSub}
        <span class="ink-row-sub ink-row-sub-hover">
          {hoverSub}
        </span>
      {/if}
      {#if status}
        <span class="ink-status-dot" data-status={status} aria-hidden="true"></span>
      {/if}
      {@render children?.()}
    </div>
  {/if}

  {#if error || warning || note || suggestions.length > 0}
    <div class="ink-row-feedback" role={error ? 'alert' : 'status'}>
      {#if error}
        <span class="ink-row-error">{error}</span>
      {:else if warning}
        <span class="ink-row-warn">{warning}</span>
      {:else if note}
        <span class="ink-row-note">{note}</span>
      {/if}
      {#if suggestions.length > 0}
        <div class="ink-row-suggestions">
          <span class="ink-suggest-label">{t('common.recommended')}</span>
          {#each suggestions as combo (combo)}
            <button
              type="button"
              class="ink-suggest-chip ink-mono ink-focus"
              onclick={() => onSelectSuggestion?.(combo)}
            >
              {formatComboReadable(combo, resolvedPlatform)}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .ink-row-wrap {
    border-top: 1px solid var(--ink-hair);
    transition: background-color 100ms ease;
  }

  .ink-row-wrap:first-child {
    border-top: 0;
  }

  .ink-row-wrap[data-recording='true'] {
    background: var(--ink-rec-bg);
  }

  .ink-row-wrap[data-shaking='true'] {
    animation: ink-shake 180ms ease-in-out;
  }

  .ink-row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 33px;
    padding: 4px 13px;
    font-size: 13px;
    color: var(--ink-text);
    box-sizing: border-box;
  }

  .ink-row-clickable {
    cursor: pointer;
  }

  /* R2-FIX-6: Group overflow:hidden에 초점 테두리가 잘리지 않도록 안쪽(inset) 링으로 그린다 */
  .ink-row-clickable.ink-focus:focus-visible {
    outline: none;
    outline-offset: 0;
    box-shadow: inset 0 0 0 2px var(--ink-accent);
  }

  .ink-row-wrap:first-child .ink-row-clickable {
    border-top-left-radius: var(--ink-r-group);
    border-top-right-radius: var(--ink-r-group);
  }

  .ink-row-wrap:last-child .ink-row-clickable {
    border-bottom-left-radius: var(--ink-r-group);
    border-bottom-right-radius: var(--ink-r-group);
  }

  .ink-row-clickable:hover {
    background: var(--ink-hover);
  }

  .ink-row-wrap[data-recording='true'] .ink-row-clickable:hover {
    background: transparent;
  }

  .ink-row-label {
    flex: 1 0 auto;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }

  .ink-row-label[data-danger='true'] {
    color: var(--ink-danger-text);
  }

  .ink-later-tag {
    display: inline-flex;
    align-items: center;
    height: 16px;
    padding: 0 5px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 500;
    color: var(--ink-sub);
    background: var(--ink-chip-bg);
  }

  .ink-row-sub {
    font-size: 11.5px;
    color: var(--ink-sub);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .ink-row-sub-hover {
    opacity: 0;
    pointer-events: none;
    transition: opacity 100ms ease;
  }

  .ink-row-wrap:hover .ink-row-sub-hover,
  .ink-row-wrap:focus-within .ink-row-sub-hover {
    opacity: 1;
  }

  .ink-row-sub[data-recording='true'] {
    color: var(--ink-accent-text);
    font-size: 11px;
  }

  .ink-status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex: none;
  }

  .ink-status-dot[data-status='ok'] {
    background: var(--ink-ok);
  }

  .ink-status-dot[data-status='attn'] {
    background: var(--ink-attn);
  }

  .ink-status-dot[data-status='danger'] {
    background: var(--ink-danger);
  }

  .ink-row-feedback {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 0 13px 8px;
    font-size: 11.5px;
  }

  .ink-row-error {
    color: var(--ink-danger-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .ink-row-warn {
    color: var(--ink-accent-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .ink-row-note {
    color: var(--ink-sub);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .ink-row-suggestions {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    flex-wrap: nowrap;
  }

  .ink-suggest-label {
    font-size: 11px;
    color: var(--ink-sub);
    white-space: nowrap;
  }

  .ink-suggest-chip {
    height: 20px;
    padding: 0 7px;
    border-radius: var(--ink-r-key);
    border: 1px solid var(--ink-line-strong);
    background: var(--ink-surface);
    color: var(--ink-text);
    font-size: 10.5px;
    white-space: nowrap;
    cursor: pointer;
  }

  .ink-suggest-chip:hover {
    border-color: var(--ink-attn);
    color: var(--ink-accent-text);
  }

  .ink-suggest-chip:active {
    opacity: 0.85;
  }
</style>
