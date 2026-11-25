<script>
  import { DropdownMenu, Separator } from 'bits-ui';

  /**
   * 트레이 팝오버 컨테이너(F03 · 10 §5).
   * - `trigger` 스니펫이 주어지면 bits-ui DropdownMenu 위에 띄운다.
   * - 단독 창(Tauri 트레이 창·미리보기)에서는 role="menu" 패널로 동작하며 ↑↓/Home/End/Esc 키보드 이동을 지원한다(첫 글자 점프 없음 — 한글 IME 보호).
   * @type {{
   *   open?: boolean,
   *   label?: string,
   *   onClose?: () => void,
   *   onOpenChange?: (open: boolean) => void,
   *   trigger?: import('svelte').Snippet,
   *   header?: import('svelte').Snippet,
   *   footer?: import('svelte').Snippet,
   *   children?: import('svelte').Snippet,
   *   class?: string,
   * }}
   */
  let {
    open = $bindable(true),
    label = 'screen-ink 메뉴',
    onClose = undefined,
    onOpenChange = undefined,
    trigger = undefined,
    header = undefined,
    footer = undefined,
    children = undefined,
    class: className = '',
  } = $props();

  let menuEl = $state(null);

  function handleMenuKeydown(e) {
    if (e.key === 'Escape') {
      e.preventDefault();
      open = false;
      onOpenChange?.(false);
      onClose?.();
      return;
    }

    if (!menuEl) return;
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp' && e.key !== 'Home' && e.key !== 'End') {
      return;
    }

    const items = Array.from(
      menuEl.querySelectorAll('[role="menuitem"]:not(:disabled), [role="menuitemcheckbox"]:not(:disabled)'),
    );
    if (items.length === 0) return;

    e.preventDefault();
    const currentIdx = items.indexOf(document.activeElement);
    let nextIdx = 0;
    if (e.key === 'ArrowDown') {
      nextIdx = currentIdx < 0 ? 0 : (currentIdx + 1) % items.length;
    } else if (e.key === 'ArrowUp') {
      nextIdx = currentIdx <= 0 ? items.length - 1 : currentIdx - 1;
    } else if (e.key === 'End') {
      nextIdx = items.length - 1;
    }
    items[nextIdx]?.focus();
  }
</script>

{#if trigger}
  <DropdownMenu.Root
    {open}
    onOpenChange={(next) => {
      open = next;
      onOpenChange?.(next);
      if (!next) onClose?.();
    }}
  >
    <DropdownMenu.Trigger>
      {@render trigger()}
    </DropdownMenu.Trigger>
    <DropdownMenu.Portal>
      <DropdownMenu.Content class="ink-popover {className}" aria-label={label} sideOffset={8}>
        {#if header}
          <div class="ink-popover-head">{@render header()}</div>
          <Separator.Root class="ink-popover-sep" />
        {/if}
        <div class="ink-popover-body">{@render children?.()}</div>
        {#if footer}
          <Separator.Root class="ink-popover-sep" />
          <div class="ink-popover-foot">{@render footer()}</div>
        {/if}
      </DropdownMenu.Content>
    </DropdownMenu.Portal>
  </DropdownMenu.Root>
{:else if open}
  <div
    bind:this={menuEl}
    role="menu"
    aria-label={label}
    tabindex="-1"
    onkeydown={handleMenuKeydown}
    class="ink-popover ink-popover-open {className}"
  >
    {#if header}
      <div class="ink-popover-head">{@render header()}</div>
      <Separator.Root class="ink-popover-sep" />
    {/if}
    <div class="ink-popover-body">{@render children?.()}</div>
    {#if footer}
      <Separator.Root class="ink-popover-sep" />
      <div class="ink-popover-foot">{@render footer()}</div>
    {/if}
  </div>
{/if}

<style>
  :global(.ink-popover) {
    width: 288px;
    box-sizing: border-box;
    padding: var(--ink-popover-pad);
    border-radius: var(--ink-r-popover);
    background: var(--ink-popover-bg);
    color: var(--ink-text);
    box-shadow: var(--ink-shadow-popover);
    font-family: var(--ink-font-sans);
    outline: none;
  }

  .ink-popover-open {
    animation: ink-popover-in 140ms cubic-bezier(0.2, 0, 0, 1);
  }

  @keyframes ink-popover-in {
    from {
      opacity: 0;
      transform: translateY(4px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .ink-popover-head {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 6px 10px 8px;
  }

  .ink-popover-body {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .ink-popover-foot {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  :global(.ink-popover-sep) {
    height: 1px;
    background: var(--ink-line-soft);
    margin: 5px 10px;
    border: 0;
  }
</style>
