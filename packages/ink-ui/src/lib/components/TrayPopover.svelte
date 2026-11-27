<script>
  import { DropdownMenu, Separator } from 'bits-ui';
  import { t } from '../i18n/index.svelte.js';
  import { resolveMenuKeyAction } from '../models/trayMenu.js';

  /**
   * 메뉴 막대 / 작업 표시줄 팝오버 컨테이너(F03 · R2-UI-TRAY · D3).
   * - 너비 252pt, Paperlogy 기반 표시 글꼴, macOS vibrancy / Windows Mica 반투명 표면.
   * - ↑↓·Home·End·Enter·Esc·첫 글자(한글 초성/음절·영어) 키보드 이동을 지원한다.
   * @type {{
   *   open?: boolean,
   *   label?: string,
   *   platform?: 'windows' | 'macos',
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
    label = '',
    platform = 'windows',
    onClose = undefined,
    onOpenChange = undefined,
    trigger = undefined,
    header = undefined,
    footer = undefined,
    children = undefined,
    class: className = '',
  } = $props();

  let menuEl = $state(null);
  let resolvedLabel = $derived(label || t('tray.menu.aria'));

  function handleMenuKeydown(e) {
    if (!menuEl && e.key !== 'Escape') return;

    const nodes = menuEl
      ? /** @type {HTMLElement[]} */ (
          Array.from(menuEl.querySelectorAll('[role="menuitem"], [role="menuitemcheckbox"]'))
        )
      : [];
    const items = nodes.map((el) => ({
      label: el.dataset.menuLabel || el.textContent || '',
      disabled: el.hasAttribute('disabled') || el.getAttribute('aria-disabled') === 'true',
    }));
    const currentIdx = nodes.indexOf(/** @type {HTMLElement} */ (document.activeElement));

    const result = resolveMenuKeyAction(
      {
        key: e.key,
        isComposing: e.isComposing,
        ctrlKey: e.ctrlKey,
        metaKey: e.metaKey,
        altKey: e.altKey,
      },
      items,
      currentIdx,
    );

    if (result.type === 'close') {
      e.preventDefault();
      open = false;
      onOpenChange?.(false);
      onClose?.();
    } else if (result.type === 'focus') {
      e.preventDefault();
      nodes[result.index]?.focus();
    } else if (result.type === 'activate') {
      e.preventDefault();
      nodes[result.index]?.click();
    }
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
      <DropdownMenu.Content
        class="ink-popover {className}"
        data-platform={platform}
        aria-label={resolvedLabel}
        sideOffset={8}
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
      </DropdownMenu.Content>
    </DropdownMenu.Portal>
  </DropdownMenu.Root>
{:else if open}
  <div
    bind:this={menuEl}
    role="menu"
    aria-label={resolvedLabel}
    data-platform={platform}
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
  /* 라이트·다크: macOS vibrancy / Windows Mica 반투명 + 흐림 기본값 (tokens.css에서 자동 전환) */
  :global(.ink-popover) {
    width: 252px;
    box-sizing: border-box;
    padding: 8px;
    border-radius: var(--ink-r-popover);
    background: var(--ink-tray-glass-bg);
    backdrop-filter: blur(22px) saturate(1.35);
    -webkit-backdrop-filter: blur(22px) saturate(1.35);
    color: var(--ink-text);
    box-shadow: var(--ink-tray-glass-shadow);
    font-family: var(--ink-font-display);
    font-size: var(--ink-text-base);
    letter-spacing: var(--ink-tracking-ko);
    outline: none;
  }

  :global(.ink-popover[data-platform='macos']) {
    background: var(--ink-tray-glass-mac);
    backdrop-filter: blur(28px) saturate(1.6);
    -webkit-backdrop-filter: blur(28px) saturate(1.6);
  }

  :global(.ink-popover[data-platform='windows']) {
    background: var(--ink-tray-glass-win);
    backdrop-filter: blur(18px) saturate(1.18);
    -webkit-backdrop-filter: blur(18px) saturate(1.18);
  }

  :global([data-theme='dark'] .ink-popover[data-platform='macos']) {
    backdrop-filter: blur(28px) saturate(1.55);
    -webkit-backdrop-filter: blur(28px) saturate(1.55);
  }

  :global([data-theme='dark'] .ink-popover[data-platform='windows']) {
    backdrop-filter: blur(18px) saturate(1.15);
    -webkit-backdrop-filter: blur(18px) saturate(1.15);
  }

  @media (prefers-reduced-transparency: reduce) {
    :global(.ink-popover),
    :global(.ink-popover[data-platform='macos']),
    :global(.ink-popover[data-platform='windows']),
    :global([data-theme='dark'] .ink-popover),
    :global([data-theme='dark'] .ink-popover[data-platform='macos']),
    :global([data-theme='dark'] .ink-popover[data-platform='windows']) {
      background: var(--ink-popover-bg);
      backdrop-filter: none;
      -webkit-backdrop-filter: none;
    }
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
    gap: 8px;
    padding: 3px 6px 5px;
  }

  .ink-popover-body {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .ink-popover-foot {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  :global(.ink-popover-sep) {
    height: 1px;
    background: var(--ink-line-soft);
    margin: 5px 6px;
    border: 0;
  }
</style>
