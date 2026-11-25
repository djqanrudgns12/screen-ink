<script>
  import { Spring } from 'svelte/motion';
  import Island from '../components/Island.svelte';
  import {
    ISLAND_SPRING,
    clearModeIsland,
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
   * 한 줄 ↔ 두 줄 ↔ 결과 전환은 스프링(단단함 .22 · 감쇠 .78)으로 폭·높이를 잇는다(10 §6).
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
  /** @type {HTMLDivElement | null} */
  let measureEl = $state(null);
  let hasMeasured = $state(false);

  const springW = new Spring(0, {
    stiffness: ISLAND_SPRING.stiffness,
    damping: ISLAND_SPRING.damping,
  });
  const springH = new Spring(0, {
    stiffness: ISLAND_SPRING.stiffness,
    damping: ISLAND_SPRING.damping,
  });

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
        queue =
          e.island?.kind === 'end'
            ? clearModeIsland(queue)
            : pushIsland(queue, e.island, { now: Date.now(), reducedMotion });
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

  $effect(() => {
    const el = measureEl;
    if (!el) {
      hasMeasured = false;
      return;
    }

    const syncSize = () => {
      const rect = el.getBoundingClientRect();
      const w = Math.ceil(rect.width);
      const h = Math.ceil(rect.height);
      if (w <= 0 || h <= 0) return;

      if (!hasMeasured || reducedMotion) {
        springW.set(w, { instant: true });
        springH.set(h, { instant: true });
        hasMeasured = true;
      } else {
        springW.target = w;
        springH.target = h;
      }
    };

    syncSize();
    if (typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(syncSize);
    ro.observe(el);
    return () => ro.disconnect();
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

  let mutedSurface = $derived(variant === 'off' || variant === 'security');
  let showHints = $derived(shouldShowHintLine(queue));
  let morphKey = $derived(
    current ? `${current.kind}:${variant}:${current.text}:${showHints ? 1 : 0}` : '',
  );
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
    <div
      class="ink-island-morph"
      data-muted={mutedSurface ? 'true' : undefined}
      style:width={hasMeasured ? `${springW.current}px` : undefined}
      style:height={hasMeasured ? `${springH.current}px` : undefined}
    >
      <div class="ink-island-measure" bind:this={measureEl}>
        {#key morphKey}
          <div
            class="ink-island-content"
            data-reveal={hasMeasured && !reducedMotion ? 'true' : undefined}
          >
            {#key queue.seq}
              <Island
                {variant}
                text={current.text}
                lead={current.lead || ''}
                detail={current.detail || ''}
                value={current.value || ''}
                valueMuted={Boolean(current.valueMuted)}
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
        {/key}
      </div>
    </div>
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

  .ink-island-morph {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    max-width: 520px;
    border-radius: 22px;
    background: var(--ink-island);
    box-shadow: var(--ink-island-shadow);
    overflow: hidden;
    pointer-events: auto;
  }

  .ink-island-morph[data-muted='true'] {
    background: var(--ink-island-muted);
  }

  .ink-island-measure {
    flex: none;
    display: inline-flex;
  }

  .ink-island-content {
    display: inline-flex;
  }

  .ink-island-content[data-reveal='true'] {
    animation: ink-island-content-in 120ms ease-out 80ms both;
  }

  .ink-island-morph :global(.ink-island) {
    background: transparent;
    box-shadow: none;
  }

  @keyframes ink-island-content-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
</style>
