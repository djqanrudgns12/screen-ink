<script>
  import { RadioGroup } from 'bits-ui';

  /**
   * @typedef {{ value: string | number, label: string, count?: number }} SegmentOption
   * @type {{
   *   options: SegmentOption[],
   *   value?: string | number,
   *   label?: string,
   *   mono?: boolean,
   *   disabled?: boolean,
   *   onChange?: (value: any) => void,
   *   class?: string,
   * }}
   */
  let {
    options = [],
    value = $bindable(''),
    label = '',
    mono = false,
    disabled = false,
    onChange = undefined,
    class: className = '',
  } = $props();

  let strValue = $derived(String(value));

  function handleChange(next) {
    const found = options.find((o) => String(o.value) === next);
    const resolved = found ? found.value : next;
    value = resolved;
    onChange?.(resolved);
  }
</script>

<RadioGroup.Root
  value={strValue}
  onValueChange={handleChange}
  orientation="horizontal"
  {disabled}
  aria-label={label || undefined}
  class="ink-seg {mono ? 'ink-mono' : ''} {className}"
>
  {#each options as opt (String(opt.value))}
    <RadioGroup.Item
      value={String(opt.value)}
      class="ink-seg-item ink-focus"
    >
      <span>{opt.label}</span>
      {#if opt.count !== undefined}
        <span class="ink-seg-count ink-mono">{opt.count}</span>
      {/if}
    </RadioGroup.Item>
  {/each}
</RadioGroup.Root>

<style>
  :global(.ink-seg) {
    display: inline-flex;
    align-items: center;
    padding: var(--ink-seg-pad);
    gap: var(--ink-seg-gap);
    border-radius: var(--ink-r-seg);
    background: var(--ink-seg-bg);
    box-shadow: var(--ink-seg-ring);
    overflow: hidden;
    flex: none;
  }

  :global(.ink-seg-item) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    height: var(--ink-seg-item-h);
    padding: 0 10px;
    border: 0;
    border-left: var(--ink-seg-item-border);
    border-radius: var(--ink-r-seg-item);
    background: transparent;
    color: var(--ink-text-2);
    font-family: inherit;
    font-size: 12px;
    font-weight: 400;
    white-space: nowrap;
    cursor: pointer;
    transition:
      background-color 150ms cubic-bezier(0.25, 0.9, 0.35, 1),
      color 150ms cubic-bezier(0.25, 0.9, 0.35, 1);
  }

  :global(.ink-seg-item:first-child) {
    border-left: 0;
  }

  :global(.ink-seg-item[data-state='checked']) {
    background: var(--ink-invert);
    color: var(--ink-on-invert);
    font-weight: 500;
    border-left-color: var(--ink-invert);
  }

  :global(.ink-seg-item:active) {
    opacity: 0.85;
  }

  .ink-seg-count {
    font-size: 11px;
    color: var(--ink-sub);
  }

  :global(.ink-seg-item[data-state='checked']) .ink-seg-count {
    color: var(--ink-key-on-invert-text);
  }
</style>
