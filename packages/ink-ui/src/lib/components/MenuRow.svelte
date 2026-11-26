<script>
  import Icon from '../icons/Icon.svelte';
  import Keycap from './Keycap.svelte';

  /**
   * 메뉴 한 줄(F01 · F03 · R2-UI-TRAY).
   * 상태: default | hover | press | selected | disabled
   * @type {{
   *   icon?: string,
   *   label: string,
   *   combo?: string,
   *   keys?: string[],
   *   platform?: 'windows' | 'macos',
   *   keyDim?: boolean,
   *   selected?: boolean,
   *   disabled?: boolean,
   *   muted?: boolean,
   *   visualState?: 'default' | 'hover' | 'press' | 'selected' | 'disabled',
   *   onclick?: () => void,
   *   trailing?: import('svelte').Snippet,
   *   class?: string,
   * }}
   */
  let {
    icon = '',
    label,
    combo = '',
    keys = undefined,
    platform = 'windows',
    keyDim = false,
    selected = false,
    disabled = false,
    muted = false,
    visualState = 'default',
    onclick = undefined,
    trailing = undefined,
    class: className = '',
  } = $props();

  let isSelected = $derived(selected || visualState === 'selected');
  let isDisabled = $derived(disabled || visualState === 'disabled');
  let keyVariant = $derived(
    isSelected
      ? 'focus'
      : isDisabled || keyDim
        ? 'dim'
        : visualState === 'hover' || visualState === 'press'
          ? 'hover'
          : 'default',
  );
</script>

<button
  type="button"
  role="menuitem"
  disabled={isDisabled}
  data-state={isSelected ? 'selected' : visualState}
  data-muted={muted ? 'true' : undefined}
  data-menu-label={label}
  {onclick}
  class="ink-menu-row {className}"
>
  {#if icon}
    <span class="ink-menu-icon">
      <Icon name={icon} size={16} accent={!isDisabled} inverted={isSelected} />
    </span>
  {/if}
  <span class="ink-menu-label">{label}</span>
  {#if trailing}
    <span class="ink-menu-trailing">{@render trailing()}</span>
  {/if}
  {#if combo || (keys && keys.length)}
    <Keycap {combo} {keys} {platform} variant={keyVariant} class="ink-menu-keycap" />
  {/if}
</button>

<style>
  .ink-menu-row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 28px;
    padding: 0 8px;
    box-sizing: border-box;
    border: 0;
    border-radius: var(--ink-r-row);
    background: transparent;
    color: var(--ink-text);
    font-family: inherit;
    font-size: var(--ink-text-base);
    font-weight: 500;
    line-height: 1.2;
    letter-spacing: var(--ink-tracking-ko);
    text-align: left;
    cursor: pointer;
    outline: none;
    transition: background-color 80ms ease-out, color 80ms ease-out;
  }

  .ink-menu-row[data-muted='true'] {
    color: var(--ink-sub);
  }

  .ink-menu-icon {
    display: inline-flex;
    align-items: center;
    color: var(--ink-text-2);
    flex: none;
  }

  .ink-menu-label {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ink-menu-trailing {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex: none;
  }

  .ink-menu-row :global(.ink-menu-keycap .ink-kbd[data-variant='default']) {
    color: var(--ink-sub);
    font-weight: 400;
  }

  .ink-menu-row:hover:not(:disabled),
  .ink-menu-row[data-state='hover'] {
    background: var(--ink-hover);
  }

  .ink-menu-row:active:not(:disabled),
  .ink-menu-row[data-state='press'] {
    background: var(--ink-press);
    opacity: 0.92;
  }

  .ink-menu-row:focus-visible,
  .ink-menu-row[data-state='selected'] {
    background: var(--ink-invert);
    color: var(--ink-on-invert);
  }

  .ink-menu-row:focus-visible .ink-menu-icon,
  .ink-menu-row[data-state='selected'] .ink-menu-icon {
    color: var(--ink-on-invert);
  }

  .ink-menu-row:focus-visible :global(.ink-kbd) {
    color: var(--ink-key-on-invert-text);
    background: var(--ink-key-on-invert-bg);
    border-color: var(--ink-key-on-invert-border);
  }

  .ink-menu-row:disabled,
  .ink-menu-row[data-state='disabled'] {
    color: var(--ink-muted);
    cursor: not-allowed;
  }

  .ink-menu-row:disabled .ink-menu-icon,
  .ink-menu-row[data-state='disabled'] .ink-menu-icon {
    color: var(--ink-muted);
  }
</style>
