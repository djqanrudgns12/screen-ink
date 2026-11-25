<script>
  import Icon from '../icons/Icon.svelte';

  /**
   * 경고·겹침·안내 한 줄 배너(F04).
   * @type {{
   *   kind?: 'conflict' | 'problem' | 'info',
   *   lead?: string,
   *   message: string,
   *   primaryLabel?: string,
   *   onPrimary?: () => void,
   *   secondaryLabel?: string,
   *   onSecondary?: () => void,
   *   class?: string,
   * }}
   */
  let {
    kind = 'conflict',
    lead = '',
    message,
    primaryLabel = '',
    onPrimary = undefined,
    secondaryLabel = '',
    onSecondary = undefined,
    class: className = '',
  } = $props();
</script>

<div role="status" class="ink-banner {className}" data-kind={kind}>
  <span class="ink-banner-icon">
    <Icon name={kind === 'info' ? 'info' : 'warning'} size={16} />
  </span>
  <p class="ink-banner-text">
    {#if lead}
      <strong>{lead}</strong>
    {/if}{message}
  </p>
  <div class="ink-banner-actions">
    {#if primaryLabel && onPrimary}
      <button type="button" class="ink-banner-btn ink-banner-primary ink-focus" onclick={onPrimary}>
        {primaryLabel}
      </button>
    {/if}
    {#if secondaryLabel && onSecondary}
      <button type="button" class="ink-banner-btn ink-banner-secondary ink-focus" onclick={onSecondary}>
        {secondaryLabel}
      </button>
    {/if}
  </div>
</div>

<style>
  .ink-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 8px 7px 12px;
    border-radius: var(--ink-r-group);
    background: var(--ink-group-bg);
    box-shadow: var(--ink-group-ring);
  }

  .ink-banner-icon {
    display: inline-flex;
    color: var(--ink-attn);
    flex: none;
  }

  .ink-banner[data-kind='problem'] .ink-banner-icon {
    color: var(--ink-danger);
  }

  .ink-banner[data-kind='info'] .ink-banner-icon {
    color: var(--ink-text-2);
  }

  .ink-banner-text {
    margin: 0;
    flex: 1;
    min-width: 0;
    font-size: 12.5px;
    color: var(--ink-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ink-banner-text strong {
    font-weight: 600;
  }

  .ink-banner-actions {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex: none;
  }

  .ink-banner-btn {
    border: 0;
    height: 26px;
    border-radius: var(--ink-r-btn);
    font-family: inherit;
    font-size: 12px;
    white-space: nowrap;
    cursor: pointer;
  }

  .ink-banner-primary {
    padding: 0 10px;
    font-weight: 500;
    background: var(--ink-invert);
    color: var(--ink-on-invert);
  }

  .ink-banner-secondary {
    padding: 0 8px;
    background: transparent;
    color: var(--ink-text-2);
  }

  .ink-banner-secondary:hover {
    background: var(--ink-hover);
  }

  .ink-banner-btn:active {
    opacity: 0.85;
  }
</style>
