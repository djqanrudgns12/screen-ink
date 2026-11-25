<script>
  import { Dialog } from 'bits-ui';

  /**
   * 가운데 확인 시트(bits-ui Dialog 기반). 위험 단추는 3초 뒤 활성(10 §5).
   * @type {{
   *   open?: boolean,
   *   title: string,
   *   description?: string,
   *   confirmLabel?: string,
   *   cancelLabel?: string,
   *   confirmDisabled?: boolean,
   *   width?: 'sm' | 'md',
   *   danger?: boolean,
   *   dangerDelaySeconds?: number,
   *   onConfirm?: () => void,
   *   onCancel?: () => void,
   *   onOpenChange?: (open: boolean) => void,
   *   children?: import('svelte').Snippet,
   * }}
   */
  let {
    open = $bindable(false),
    title,
    description = '',
    confirmLabel = '확인',
    cancelLabel = '취소',
    confirmDisabled = false,
    width = 'sm',
    danger = false,
    dangerDelaySeconds = 3,
    onConfirm = undefined,
    onCancel = undefined,
    onOpenChange = undefined,
    children = undefined,
  } = $props();

  let remaining = $state(0);

  $effect(() => {
    if (!open || !danger || dangerDelaySeconds <= 0) {
      remaining = 0;
      return;
    }
    remaining = dangerDelaySeconds;
    const timer = setInterval(() => {
      remaining = Math.max(0, remaining - 1);
      if (remaining === 0) clearInterval(timer);
    }, 1000);
    return () => clearInterval(timer);
  });

  function handleOpenChange(next) {
    open = next;
    onOpenChange?.(next);
    if (!next) onCancel?.();
  }

  function handleConfirm() {
    if (confirmDisabled || (danger && remaining > 0)) return;
    onConfirm?.();
    open = false;
    onOpenChange?.(false);
  }
</script>

<Dialog.Root {open} onOpenChange={handleOpenChange}>
  <Dialog.Portal>
    <Dialog.Overlay class="ink-sheet-overlay" />
    <Dialog.Content class="ink-sheet-content" data-width={width}>
      <Dialog.Title class="ink-sheet-title">{title}</Dialog.Title>
      {#if description}
        <Dialog.Description class="ink-sheet-desc">{description}</Dialog.Description>
      {/if}
      {#if children}
        <div class="ink-sheet-body">
          {@render children()}
        </div>
      {/if}
      <div class="ink-sheet-footer">
        <Dialog.Close class="ink-sheet-btn ink-sheet-cancel ink-focus">
          {cancelLabel}
        </Dialog.Close>
        {#if confirmLabel}
          <button
            type="button"
            class="ink-sheet-btn ink-focus"
            class:ink-sheet-danger={danger}
            class:ink-sheet-primary={!danger}
            disabled={confirmDisabled || (danger && remaining > 0)}
            onclick={handleConfirm}
          >
            {confirmLabel}{#if danger && remaining > 0} <span class="ink-mono">({remaining})</span>{/if}
          </button>
        {/if}
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.ink-sheet-overlay) {
    position: fixed;
    inset: 0;
    z-index: 60;
    background: var(--ink-sheet-scrim);
  }

  :global(.ink-sheet-content) {
    position: fixed;
    left: 50%;
    top: 50%;
    z-index: 61;
    width: min(380px, calc(100vw - 32px));
    transform: translate(-50%, -50%);
    box-sizing: border-box;
    padding: 20px 22px 18px;
    border-radius: var(--ink-r-window);
    background: var(--ink-surface);
    color: var(--ink-text);
    box-shadow: var(--ink-shadow-window);
    font-family: var(--ink-font-sans);
    outline: none;
  }

  :global(.ink-sheet-content[data-width='md']) {
    width: min(440px, calc(100vw - 32px));
  }

  :global(.ink-sheet-title) {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--ink-text);
  }

  :global(.ink-sheet-desc) {
    margin: 8px 0 0;
    font-size: 13px;
    line-height: 1.5;
    color: var(--ink-text-2);
  }

  .ink-sheet-body {
    margin-top: 14px;
  }

  .ink-sheet-footer {
    margin-top: 18px;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  :global(.ink-sheet-btn) {
    height: 30px;
    padding: 0 14px;
    border: 0;
    border-radius: var(--ink-r-btn);
    font-family: inherit;
    font-size: 12.5px;
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
  }

  :global(.ink-sheet-cancel) {
    background: var(--ink-btn-sec-bg);
    color: var(--ink-text);
    box-shadow: var(--ink-btn-sec-shadow);
  }

  .ink-sheet-primary {
    background: var(--ink-invert);
    color: var(--ink-on-invert);
  }

  .ink-sheet-danger {
    background: var(--ink-danger);
    color: var(--ink-island-text);
  }

  :global(.ink-sheet-btn:disabled) {
    opacity: 0.45;
    cursor: not-allowed;
  }

  :global(.ink-sheet-btn:active:not(:disabled)) {
    opacity: 0.85;
  }
</style>
