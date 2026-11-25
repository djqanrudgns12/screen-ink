<script>
  import { Switch as BitsSwitch } from 'bits-ui';

  /**
   * @type {{
   *   checked?: boolean,
   *   disabled?: boolean,
   *   label?: string,
   *   onCheckedChange?: (checked: boolean) => void,
   *   class?: string,
   * }}
   */
  let {
    checked = $bindable(false),
    disabled = false,
    label = '',
    onCheckedChange = undefined,
    class: className = '',
  } = $props();

  function handleChange(next) {
    checked = next;
    onCheckedChange?.(next);
  }
</script>

<BitsSwitch.Root
  {checked}
  {disabled}
  aria-label={label || undefined}
  onCheckedChange={handleChange}
  class="ink-switch ink-focus {className}"
>
  <BitsSwitch.Thumb class="ink-switch-thumb" />
</BitsSwitch.Root>

<style>
  :global(.ink-switch) {
    position: relative;
    display: inline-flex;
    align-items: center;
    width: 32px;
    height: 18px;
    padding: 2px;
    box-sizing: border-box;
    border: 0;
    border-radius: 9px;
    background: var(--ink-switch-off-bg);
    cursor: pointer;
    flex: none;
    transition: background-color 150ms cubic-bezier(0.25, 0.9, 0.35, 1);
  }

  :global(.ink-switch[data-state='checked']) {
    background: var(--ink-switch-on-bg);
  }

  :global(.ink-switch:disabled) {
    opacity: 0.45;
    cursor: not-allowed;
  }

  :global(.ink-switch-thumb) {
    display: block;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--ink-switch-off-thumb);
    box-shadow: var(--ink-shadow-thumb);
    transform: translateX(0);
    transition:
      transform 150ms cubic-bezier(0.25, 0.9, 0.35, 1),
      background-color 150ms cubic-bezier(0.25, 0.9, 0.35, 1);
  }

  :global(.ink-switch[data-state='checked'] .ink-switch-thumb) {
    transform: translateX(14px);
    background: var(--ink-switch-on-thumb);
    box-shadow: none;
  }
</style>
