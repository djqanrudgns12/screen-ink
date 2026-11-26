<script>
  import Icon from '../icons/Icon.svelte';

  /**
   * 선택 툴바(R-CAP-27, 04 S08): 확정한 범위 옆 다크 알약. 위치는 부모가 `placeToolbar`로 정한다.
   * 단추를 눌러도 포인터 사건이 오버레이로 가지 않는다(범위를 건드리지 않음).
   * @type {{
   *   actions: ReadonlyArray<{ id: string, icon: string, label: string, command: string, primary?: boolean }>,
   *   tooltips?: Record<string, string>,
   *   x?: number,
   *   y?: number,
   *   onCommand?: (command: string) => void,
   *   onHover?: (over: boolean) => void,
   *   width?: number,
   *   height?: number,
   * }}
   */
  let { actions, tooltips = {}, x = 0, y = 0, onCommand = () => {}, onHover = () => {}, width = $bindable(0), height = $bindable(0) } = $props();

  /** @param {Event} e */
  function stop(e) {
    e.stopPropagation();
  }
</script>

<div
  class="ink-toolbar"
  role="toolbar"
  tabindex="-1"
  aria-label="캡처 도구"
  style:transform={`translate3d(${x}px, ${y}px, 0)`}
  bind:offsetWidth={width}
  bind:offsetHeight={height}
  onpointerenter={() => onHover(true)}
  onpointerleave={() => onHover(false)}
  onpointerdown={stop}
  onpointermove={stop}
  onpointerup={stop}
  ondblclick={stop}
  oncontextmenu={(e) => {
    e.preventDefault();
    e.stopPropagation();
  }}
>
  {#each actions as a (a.id)}
    <button
      type="button"
      class="ink-toolbar-btn"
      class:primary={a.primary}
      data-action={a.id}
      title={tooltips[a.id] ?? a.label}
      aria-label={tooltips[a.id] ?? a.label}
      onclick={(e) => {
        e.stopPropagation();
        onCommand(a.command);
      }}
    >
      <Icon name={a.icon} size={20} accent={false} />
    </button>
  {/each}
</div>

<style>
  .ink-toolbar {
    position: absolute;
    left: 0;
    top: 0;
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 40px;
    padding: 0 4px;
    box-sizing: border-box;
    border-radius: var(--ink-r-island);
    background: var(--ink-island);
    color: var(--ink-island-text);
    box-shadow: var(--ink-island-shadow);
    cursor: default;
    pointer-events: auto;
    will-change: transform;
    z-index: 5;
  }

  .ink-toolbar-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--ink-island-text);
    cursor: pointer;
  }

  .ink-toolbar-btn:hover {
    background: var(--ink-island-key-bg);
  }

  .ink-toolbar-btn:focus-visible {
    outline: 2px solid var(--ink-accent);
    outline-offset: 1px;
  }

  .ink-toolbar-btn.primary {
    background: var(--ink-accent);
    color: var(--ink-island-btn-text);
  }

  .ink-toolbar-btn.primary:hover {
    filter: brightness(1.08);
  }
</style>
