<script>
  import Icon from '../icons/Icon.svelte';
  import Keycap from './Keycap.svelte';

  /**
   * 화면 위 검은 알약 섬(S17 / F18).
   * @type {{
   *   variant?: 'region' | 'window' | 'record' | 'draw' | 'zoom' | 'board' | 'result' | 'delay' | 'pin' | 'problem' | 'off' | 'security' | 'info',
   *   text: string,
   *   lead?: string,
   *   detail?: string,
   *   value?: string,
   *   valueMuted?: boolean,
   *   keys?: string[],
   *   combo?: string,
   *   platform?: 'windows' | 'macos',
   *   keyHint?: string,
   *   countdown?: number | null,
   *   penColor?: 'R' | 'Y' | 'G' | 'O' | 'B' | 'L' | 'W' | 'K',
   *   penWidth?: string,
   *   micOff?: boolean,
   *   button?: { label: string, key?: string, stop?: boolean, onClick?: () => void } | null,
   *   secondaryButton?: { label: string, key?: string, onClick?: () => void } | null,
   *   hints?: { key: string, label: string }[],
   *   showHints?: boolean,
   *   progressDurationMs?: number | null,
   *   paused?: boolean,
   *   onmouseenter?: () => void,
   *   onmouseleave?: () => void,
   *   thumbnail?: import('svelte').Snippet,
   *   class?: string,
   * }}
   */
  let {
    variant = 'info',
    text,
    lead = '',
    detail = '',
    value = '',
    valueMuted = false,
    keys = undefined,
    combo = '',
    platform = 'windows',
    keyHint = '',
    countdown = null,
    penColor = 'R',
    penWidth = '',
    micOff = false,
    button = null,
    secondaryButton = null,
    hints = [],
    showHints = false,
    progressDurationMs = null,
    paused = false,
    onmouseenter = undefined,
    onmouseleave = undefined,
    thumbnail = undefined,
    class: className = '',
  } = $props();

  let mutedSurface = $derived(variant === 'off' || variant === 'security');
  let hasSecondLine = $derived(showHints && Array.isArray(hints) && hints.length > 0);
  let hasThumb = $derived(Boolean(thumbnail));
  let iconName = $derived.by(() => {
    if (variant === 'region') return 'region';
    if (variant === 'window') return 'window';
    if (variant === 'zoom') return 'zoom';
    if (variant === 'board') return 'board';
    if (variant === 'pin') return 'pin';
    if (variant === 'problem') return 'warning';
    if (variant === 'security') return 'lock';
    if (variant === 'result') return 'check';
    return '';
  });
</script>

<div
  role="status"
  aria-live={variant === 'problem' ? 'assertive' : 'polite'}
  class="ink-island {className}"
  data-variant={variant}
  data-muted={mutedSurface ? 'true' : undefined}
  data-twoline={hasSecondLine ? 'true' : undefined}
  data-thumb={hasThumb ? 'true' : undefined}
  {onmouseenter}
  {onmouseleave}
>
  <div class="ink-island-row">
    {#if thumbnail}
      <div class="ink-island-thumb">{@render thumbnail()}</div>
    {/if}

    {#if variant === 'delay' && countdown !== null}
      <span class="ink-island-countdown ink-mono">{countdown}</span>
    {:else if variant === 'record'}
      <span class="ink-island-rec-dot" aria-hidden="true"></span>
    {:else if variant === 'draw'}
      <span
        class="ink-island-pen-dot"
        style:background={`var(--ink-pen-${penColor})`}
        aria-hidden="true"
      ></span>
    {:else if variant === 'off'}
      <span class="ink-island-off-dot" aria-hidden="true"></span>
    {:else if iconName}
      <span
        class="ink-island-icon"
        data-tone={variant === 'result' ? 'ok' : variant === 'pin' || variant === 'problem' ? 'accent' : variant === 'security' ? 'sub' : 'light'}
      >
        <Icon name={iconName} size={16} inverted={true} />
      </span>
    {/if}

    <span class="ink-island-main">
      {#if lead}
        <strong>{lead}</strong>{text}
      {:else if variant === 'region' || variant === 'window' || variant === 'delay' || variant === 'zoom' || variant === 'board'}
        <strong>{text}</strong>
      {:else}
        {text}
      {/if}
    </span>

    {#if penWidth}
      <span class="ink-island-val ink-mono" data-muted="true">{penWidth}</span>
    {/if}

    {#if detail}
      <span class="ink-island-detail">{detail}</span>
    {/if}

    {#if value}
      <span class="ink-island-val ink-mono" data-muted={valueMuted ? 'true' : undefined}>{value}</span>
    {/if}

    {#if variant === 'record' && micOff}
      <span class="ink-island-icon" data-tone="sub">
        <Icon name="mic-off" size={16} label="마이크 꺼짐" />
      </span>
    {/if}

    {#if combo || (keys && keys.length)}
      <Keycap {combo} {keys} {platform} variant="island" />
    {/if}

    {#if keyHint}
      <span class="ink-island-detail">{keyHint}</span>
    {/if}

    {#if button}
      <button
        type="button"
        class="ink-island-btn ink-island-btn-primary ink-focus"
        onclick={button.onClick}
      >
        {#if button.stop}
          <span class="ink-island-stop-sq" aria-hidden="true"></span>
        {/if}
        <span>{button.label}</span>
        {#if button.key}
          <span class="ink-island-btn-key ink-mono">{button.key}</span>
        {/if}
      </button>
    {/if}

    {#if secondaryButton}
      <button
        type="button"
        class="ink-island-btn ink-island-btn-sec ink-focus"
        onclick={secondaryButton.onClick}
      >
        <span>{secondaryButton.label}</span>
        {#if secondaryButton.key}
          <span class="ink-island-btn-key ink-mono">{secondaryButton.key}</span>
        {/if}
      </button>
    {/if}
  </div>

  {#if hasSecondLine}
    <div class="ink-island-hints">
      {#each hints as h, i (i)}
        <span class="ink-island-hint">
          <Keycap label={h.key} variant="island" />
          <span>{h.label}</span>
        </span>
      {/each}
    </div>
  {/if}

  {#if progressDurationMs && progressDurationMs > 0}
    <span class="ink-island-progress" aria-hidden="true">
      <span
        class="ink-island-progress-bar"
        style:animation-duration={`${progressDurationMs}ms`}
        style:animation-play-state={paused ? 'paused' : 'running'}
      ></span>
    </span>
  {/if}
</div>

<style>
  .ink-island {
    position: relative;
    display: inline-flex;
    flex-direction: column;
    gap: 8px;
    max-width: 520px;
    min-height: 38px;
    padding: 0 14px;
    box-sizing: border-box;
    border-radius: var(--ink-r-island);
    background: var(--ink-island);
    color: var(--ink-island-text);
    box-shadow: var(--ink-island-shadow);
    font-family: var(--ink-font-sans);
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    pointer-events: auto;
  }

  .ink-island[data-twoline='true'] {
    border-radius: 22px;
    padding: 10px 16px;
  }

  .ink-island[data-thumb='true'] {
    padding: 5px 6px 5px 5px;
  }

  .ink-island[data-muted='true'] {
    background: var(--ink-island-muted);
    color: var(--ink-island-text-2);
  }

  .ink-island-row {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    min-height: 38px;
  }

  .ink-island[data-twoline='true'] .ink-island-row {
    min-height: 20px;
  }

  .ink-island[data-thumb='true'] .ink-island-row {
    min-height: 30px;
  }

  .ink-island-thumb {
    width: 44px;
    height: 28px;
    border-radius: 14px 6px 6px 14px;
    background: var(--ink-island-key-bg);
    overflow: hidden;
    flex: none;
    display: grid;
    place-items: center;
  }

  .ink-island-icon {
    display: inline-flex;
    align-items: center;
    flex: none;
    color: var(--ink-island-text);
  }

  .ink-island-icon[data-tone='ok'] {
    color: var(--ink-island-ok);
  }

  .ink-island-icon[data-tone='accent'] {
    color: var(--ink-accent);
  }

  .ink-island-icon[data-tone='sub'] {
    color: var(--ink-island-sub);
  }

  .ink-island-main {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }

  .ink-island-main strong {
    font-weight: 600;
  }

  .ink-island-detail {
    font-size: 12px;
    color: var(--ink-island-sub);
  }

  .ink-island-val {
    font-size: 11.5px;
    color: var(--ink-island-accent-text);
  }

  .ink-island-val[data-muted='true'] {
    color: var(--ink-island-sub);
  }

  .ink-island-countdown {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    box-shadow: inset 0 0 0 2px var(--ink-accent);
    font-size: 13px;
    color: var(--ink-accent);
    flex: none;
  }

  .ink-island-rec-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--ink-island-danger);
    box-shadow: 0 0 0 4px var(--ink-island-danger-glow);
    flex: none;
    animation: ink-pulse-rec 1.6s ease-in-out infinite;
  }

  .ink-island-pen-dot {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    box-shadow: 0 0 0 2px var(--ink-island), 0 0 0 3.5px var(--ink-island-text);
    flex: none;
  }

  .ink-island-off-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1.5px var(--ink-island-sub);
    flex: none;
  }

  .ink-island-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 11px;
    border: 0;
    border-radius: 13px;
    font-family: inherit;
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
    flex: none;
  }

  .ink-island-btn-primary {
    background: var(--ink-island-btn-bg);
    color: var(--ink-island-btn-text);
  }

  .ink-island-btn-sec {
    background: var(--ink-island-key-bg);
    color: var(--ink-island-text);
  }

  .ink-island-btn:active {
    opacity: 0.85;
  }

  .ink-island-stop-sq {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    background: var(--ink-island-btn-text);
  }

  .ink-island-btn-key {
    font-size: 10px;
    opacity: 0.7;
  }

  .ink-island-hints {
    display: flex;
    align-items: center;
    gap: 14px;
    font-size: 12px;
    color: var(--ink-island-text-2);
  }

  .ink-island-hint {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .ink-island-progress {
    position: absolute;
    left: 18px;
    right: 18px;
    bottom: 2px;
    height: 2px;
    border-radius: 1px;
    background: var(--ink-island-key-bg);
    overflow: hidden;
  }

  .ink-island-progress-bar {
    display: block;
    width: 100%;
    height: 100%;
    background: var(--ink-accent);
    transform-origin: left center;
    animation: ink-progress-shrink linear forwards;
  }

  @keyframes ink-progress-shrink {
    from {
      transform: scaleX(1);
    }
    to {
      transform: scaleX(0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .ink-island-progress {
      display: none;
    }
  }
</style>
