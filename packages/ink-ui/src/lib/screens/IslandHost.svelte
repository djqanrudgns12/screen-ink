<script>
  import Island from '../components/Island.svelte';
  import {
    createIslandState,
    dismissIsland,
    getAutoDismissMs,
    pushIsland,
    setHelpOpen,
    setIslandHover,
    shouldShowHintLine,
  } from '../models/islandQueue.js';

  /**
   * S17 섬 호스트 — 한 번에 하나, 1.2초 자동 닫힘(올리면 멈춤), 안내 줄은 처음 3번 또는 '?' 때만.
   * @type {{
   *   client: import('@screen-ink/client').InkClient,
   *   position?: 'fixed' | 'inline',
   *   class?: string,
   * }}
   */
  let { client, position = 'fixed', class: className = '' } = $props();

  let queue = $state(createIslandState());
  let platform = $state('windows');
  let reducedMotion = $state(false);

  $effect(() => {
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return;
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    reducedMotion = mq.matches;
    const onChange = (e) => {
      reducedMotion = e.matches;
    };
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  });

  $effect(() => {
    let alive = true;
    client.getState().then((s) => {
      if (alive && s?.platform) platform = s.platform;
    });
    const off = client.subscribe((e) => {
      if (e.type === 'state' && e.state?.platform) {
        platform = e.state.platform;
      } else if (e.type === 'island') {
        queue = pushIsland(queue, e.island, { now: Date.now(), reducedMotion });
      }
    });
    return () => {
      alive = false;
      off();
    };
  });

  $effect(() => {
    const exp = queue.expiresAt;
    if (exp === null || queue.hovering) return;
    const delay = Math.max(0, exp - Date.now());
    const timer = setTimeout(() => {
      queue = dismissIsland(queue);
    }, delay);
    return () => clearTimeout(timer);
  });

  function handleKeydown(e) {
    const tag = e.target?.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || e.target?.isContentEditable) return;
    if (e.key === '?' && queue.current?.hints?.length) {
      queue = setHelpOpen(queue, !queue.helpOpen);
    }
  }

  let current = $derived(queue.current);
  let variant = $derived.by(() => {
    if (!current) return 'info';
    if (current.variant) return current.variant;
    if (current.kind === 'result') return 'result';
    if (current.kind === 'problem') return 'problem';
    if (current.kind === 'off') return 'off';
    if (current.kind === 'mode') return current.mode || 'region';
    return 'info';
  });

  let showHints = $derived(shouldShowHintLine(queue));
  let durationMs = $derived(
    current && current.kind !== 'mode'
      ? getAutoDismissMs(current, { hasActiveMode: Boolean(queue.activeMode), reducedMotion })
      : null,
  );

  let actionButton = $derived(
    current?.button
      ? {
          label: current.button.label,
          key: current.button.key,
          stop: current.button.stop,
          onClick: async () => {
            const act = current.button.action;
            if (act === 'undo') {
              await client.undo();
            } else if (act) {
              await client.run(act);
            }
            if (current.kind !== 'mode') {
              queue = dismissIsland(queue);
            }
          },
        }
      : null,
  );
</script>

<svelte:window onkeydown={handleKeydown} />

{#if current}
  <div class="ink-island-host {className}" data-position={position}>
    {#key queue.seq}
      <Island
        {variant}
        text={current.text}
        lead={current.lead || ''}
        detail={current.detail || ''}
        value={current.value || ''}
        combo={current.combo || ''}
        keys={current.keys}
        {platform}
        keyHint={current.keyHint || ''}
        countdown={current.countdown ?? null}
        penColor={current.penColor || 'R'}
        penWidth={current.penWidth || ''}
        micOff={Boolean(current.micOff)}
        button={actionButton}
        hints={current.hints || []}
        {showHints}
        progressDurationMs={current.kind === 'result' ? durationMs : null}
        paused={queue.hovering}
        onmouseenter={() => (queue = setIslandHover(queue, true, { now: Date.now(), reducedMotion }))}
        onmouseleave={() => (queue = setIslandHover(queue, false, { now: Date.now(), reducedMotion }))}
      />
    {/key}
  </div>
{/if}

<style>
  .ink-island-host {
    display: flex;
    justify-content: center;
    pointer-events: none;
    z-index: 50;
  }

  .ink-island-host[data-position='fixed'] {
    position: fixed;
    left: 0;
    right: 0;
    top: 14px;
  }
</style>
