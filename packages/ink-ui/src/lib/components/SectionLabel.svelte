<script>
  /**
   * @type {{
   *   title: string,
   *   tag?: string,
   *   actionLabel?: string,
   *   onAction?: () => void,
   *   class?: string,
   * }}
   */
  let {
    title,
    tag = '',
    actionLabel = '',
    onAction = undefined,
    class: className = '',
  } = $props();
</script>

<div class="ink-section-label {className}">
  <span class="ink-section-title">
    {title}
    {#if tag}
      <span class="ink-section-tag ink-mono">{tag}</span>
    {/if}
  </span>
  {#if actionLabel && onAction}
    <button type="button" class="ink-section-action ink-focus" onclick={onAction}>
      {actionLabel}
    </button>
  {/if}
</div>

<style>
  .ink-section-label {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin: 0 4px 6px;
    gap: 8px;
  }

  .ink-section-title {
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.04em;
    color: var(--ink-sub);
    white-space: nowrap;
  }

  .ink-section-tag {
    margin-left: 4px;
    font-size: 10px;
    color: var(--ink-sub);
  }

  .ink-section-action {
    border: 0;
    background: none;
    padding: 0 2px;
    border-radius: 4px;
    font-family: inherit;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--ink-accent-text);
    white-space: nowrap;
    cursor: pointer;
  }

  .ink-section-action:hover {
    text-decoration: underline;
  }

  .ink-section-action:active {
    opacity: 0.85;
  }
</style>
