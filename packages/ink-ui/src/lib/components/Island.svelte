<script>
  import { keycaps } from '@screen-ink/client';
  import Icon from '../icons/Icon.svelte';
  import { locale, t } from '../i18n/index.svelte.js';
  import {
    DRAW_PALETTE_EXPAND_MS,
    formatBoardIslandMeta,
    getDrawPaletteSwatches,
    shouldExpandDrawPalette,
  } from '../models/drawUi.js';
  import { extractHexColor, formatCapturePath, shouldOmitButtonKeys } from '../models/islandQueue.js';
  import Keycap from './Keycap.svelte';

  /**
   * 화면 위 검은 알약 섬(S17 / F09 / F10 / F18).
   * @type {{
   *   variant?: 'region' | 'window' | 'record' | 'draw' | 'zoom' | 'board' | 'result' | 'delay' | 'pin' | 'problem' | 'off' | 'security' | 'info',
   *   text: string,
   *   lead?: string,
   *   detail?: string,
   *   value?: string,
   *   valueMuted?: boolean,
   *   path?: string[],
   *   level?: number,
   *   tags?: string[],
   *   thumbUrl?: string,
   *   keys?: string[],
   *   combo?: string,
   *   platform?: 'windows' | 'macos',
   *   keyHint?: string,
   *   countdown?: number | null,
   *   penColor?: 'R' | 'Y' | 'G' | 'O' | 'B' | 'L' | 'W' | 'K',
   *   penWidth?: string,
   *   paletteExpanded?: boolean,
   *   idleDim?: boolean,
   *   avoid?: boolean,
   *   onSelectColor?: (code: 'R' | 'Y' | 'G' | 'O' | 'B' | 'L' | 'W' | 'K') => void,
   *   micOff?: boolean,
   *   button?: { label: string, key?: string, stop?: boolean, onClick?: () => void } | null,
   *   secondaryButton?: { label: string, key?: string, onClick?: () => void } | null,
   *   chip?: { label: string, hex?: string, onClick?: () => void } | null,
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
    path = undefined,
    level = -1,
    tags = undefined,
    thumbUrl = '',
    keys = undefined,
    combo = '',
    platform = 'windows',
    keyHint = '',
    countdown = null,
    penColor = undefined,
    penWidth = '',
    paletteExpanded = false,
    idleDim = false,
    avoid = false,
    onSelectColor = undefined,
    micOff = false,
    button = null,
    secondaryButton = null,
    chip = null,
    hints = [],
    showHints = false,
    progressDurationMs = null,
    paused = false,
    onmouseenter = undefined,
    onmouseleave = undefined,
    thumbnail = undefined,
    class: className = '',
  } = $props();

  let thumbFailed = $state(false);
  /** @type {HTMLDivElement | null} */
  let islandEl = $state(null);
  let overflowDetected = $state(false);
  let paletteHovered = $state(false);
  let paletteTransient = $state(false);
  /** @type {string | null} */
  let prevPenColor = null;
  /** @type {ReturnType<typeof setTimeout> | null} */
  let paletteTimer = null;

  function schedulePaletteCollapse() {
    if (paletteTimer !== null) clearTimeout(paletteTimer);
    paletteTimer = setTimeout(() => {
      paletteTimer = null;
      paletteTransient = false;
    }, DRAW_PALETTE_EXPAND_MS);
  }

  function handlePaletteEnter() {
    paletteHovered = true;
    if (paletteTimer !== null) {
      clearTimeout(paletteTimer);
      paletteTimer = null;
    }
  }

  function handlePaletteLeave() {
    paletteHovered = false;
    paletteTransient = true;
    schedulePaletteCollapse();
  }

  $effect(() => {
    const color = activePenColor;
    const v = variant;
    if (shouldExpandDrawPalette(prevPenColor, color, v)) {
      paletteTransient = true;
      schedulePaletteCollapse();
    }
    prevPenColor = color ?? null;
  });

  $effect(() => {
    return () => {
      if (paletteTimer !== null) {
        clearTimeout(paletteTimer);
        paletteTimer = null;
      }
    };
  });
  $effect(() => {
    void thumbUrl;
    thumbFailed = false;
  });
  $effect(() => {
    void text;
    void detail;
    void value;
    void thumbUrl;
    void button;
    void secondaryButton;
    void chip;
    overflowDetected = false;
  });
  $effect(() => {
    const el = islandEl;
    if (!el) return;
    const checkOverflow = () => {
      if (!overflowDetected && el.clientWidth > 0 && el.scrollWidth > el.clientWidth + 1) {
        overflowDetected = true;
      }
    };
    checkOverflow();
    if (typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(checkOverflow);
    ro.observe(el);
    return () => ro.disconnect();
  });

  let mutedSurface = $derived(variant === 'off' || variant === 'security');
  let activePenColor = $derived(penColor ?? (variant === 'draw' ? 'R' : undefined));
  let hasPenPalette = $derived((variant === 'draw' || variant === 'board') && Boolean(activePenColor));
  let paletteSwatches = $derived(getDrawPaletteSwatches(variant === 'board', locale.lang));
  let showExpandedPalette = $derived(hasPenPalette && (paletteExpanded || paletteHovered || paletteTransient));
  let boardMeta = $derived(variant === 'board' ? formatBoardIslandMeta({ text, detail, value }) : null);
  let hasSecondLine = $derived(showHints && Array.isArray(hints) && hints.length > 0);
  let hasThumb = $derived(Boolean(thumbnail || (thumbUrl && !thumbFailed)));
  let pathItems = $derived(formatCapturePath(path, level));
  let isResultCard = $derived(variant === 'result' && Boolean(detail));
  let resultMeta = $derived([value, detail].filter(Boolean).join(' · '));
  let chipHex = $derived(chip ? (extractHexColor(chip.hex) ?? extractHexColor(chip.label)) : null);
  let omitButtonKeys = $derived(
    shouldOmitButtonKeys({ button, secondaryButton, chip, overflow: overflowDetected }),
  );
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

  /** @param {string | undefined} raw */
  function formatButtonKey(raw) {
    if (!raw) return '';
    const parts = keycaps(raw, platform);
    return parts.join(platform === 'macos' ? '' : ' ');
  }
</script>

<div
  bind:this={islandEl}
  role="status"
  aria-live={variant === 'problem' ? 'assertive' : 'polite'}
  class="ink-island {className}"
  data-variant={variant}
  data-muted={mutedSurface ? 'true' : undefined}
  data-twoline={hasSecondLine ? 'true' : undefined}
  data-thumb={hasThumb ? 'true' : undefined}
  data-result-card={isResultCard ? 'true' : undefined}
  data-compact-keys={omitButtonKeys ? 'true' : undefined}
  data-idle-dim={idleDim ? 'true' : undefined}
  data-avoid={avoid ? 'true' : undefined}
  data-palette-open={showExpandedPalette ? 'true' : undefined}
  {onmouseenter}
  {onmouseleave}
>
  <div class="ink-island-row">
    {#if thumbnail}
      <div class="ink-island-thumb">{@render thumbnail()}</div>
    {:else if thumbUrl && !thumbFailed}
      <div class="ink-island-thumb">
        <img
          class="ink-island-thumb-img"
          src={thumbUrl}
          alt=""
          draggable="false"
          onerror={() => (thumbFailed = true)}
        />
      </div>
    {/if}

    {#if isResultCard}
      <div class="ink-island-result-body">
        <span class="ink-island-result-title">
          <span class="ink-island-icon" data-tone="ok">
            <Icon name="check" size={14} inverted={true} />
          </span>
          <strong>{text}</strong>
        </span>
        {#if resultMeta}
          <span class="ink-island-result-meta ink-mono">{resultMeta}</span>
        {/if}
      </div>
    {:else}
      {#if variant === 'delay' && countdown !== null}
        <span class="ink-island-countdown ink-mono">{countdown}</span>
      {:else if variant === 'record'}
        <span class="ink-island-rec-dot" aria-hidden="true"></span>
      {:else if variant === 'draw'}
        <span
          class="ink-island-palette"
          role="group"
          aria-label={t('islandUi.draw.penColorAria')}
          onmouseenter={handlePaletteEnter}
          onmouseleave={handlePaletteLeave}
        >
          {#if showExpandedPalette}
            <span class="ink-island-swatches">
              {#each paletteSwatches as sw (sw.code)}
                {#if onSelectColor}
                  <button
                    type="button"
                    class="ink-island-swatch ink-mono ink-focus"
                    aria-label={`${sw.label} (${sw.code})`}
                    aria-pressed={activePenColor === sw.code}
                    data-selected={activePenColor === sw.code ? 'true' : undefined}
                    data-bordered={sw.needsBorder ? 'true' : undefined}
                    style:background={`var(--ink-pen-${sw.code})`}
                    style:color={sw.lightFg ? 'var(--ink-pen-fg-light)' : 'var(--ink-pen-fg-dark)'}
                    onclick={() => onSelectColor?.(sw.code)}
                  >
                    {sw.code}
                  </button>
                {:else}
                  <span
                    class="ink-island-swatch ink-mono"
                    data-selected={activePenColor === sw.code ? 'true' : undefined}
                    data-bordered={sw.needsBorder ? 'true' : undefined}
                    style:background={`var(--ink-pen-${sw.code})`}
                    style:color={sw.lightFg ? 'var(--ink-pen-fg-light)' : 'var(--ink-pen-fg-dark)'}
                    aria-hidden="true"
                  >
                    {sw.code}
                  </span>
                {/if}
              {/each}
            </span>
          {:else}
            <span
              class="ink-island-pen-dot"
              data-bordered={activePenColor === 'K' ? 'true' : undefined}
              style:background={`var(--ink-pen-${activePenColor})`}
              aria-hidden="true"
            ></span>
          {/if}
        </span>
      {:else if variant === 'off'}
        <span class="ink-island-off-dot" aria-hidden="true"></span>
      {:else if iconName}
        <span
          class="ink-island-icon"
          data-tone={variant === 'result' ? 'ok' : variant === 'pin' || variant === 'problem' ? 'accent' : variant === 'security' ? 'sub' : 'light'}
        >
          <Icon name={iconName} size={16} inverted={true} />
        </span>
        {#if variant === 'board' && hasPenPalette}
          <span
            class="ink-island-palette"
            role="group"
            aria-label={t('islandUi.board.penColorAria')}
            onmouseenter={handlePaletteEnter}
            onmouseleave={handlePaletteLeave}
          >
            {#if showExpandedPalette}
              <span class="ink-island-swatches">
                {#each paletteSwatches as sw (sw.code)}
                  {#if onSelectColor}
                    <button
                      type="button"
                      class="ink-island-swatch ink-mono ink-focus"
                      aria-label={`${sw.label} (${sw.code})`}
                      aria-pressed={activePenColor === sw.code}
                      data-selected={activePenColor === sw.code ? 'true' : undefined}
                      data-bordered={sw.needsBorder ? 'true' : undefined}
                      style:background={`var(--ink-pen-${sw.code})`}
                      style:color={sw.lightFg ? 'var(--ink-pen-fg-light)' : 'var(--ink-pen-fg-dark)'}
                      onclick={() => onSelectColor?.(sw.code)}
                    >
                      {sw.code}
                    </button>
                  {:else}
                    <span
                      class="ink-island-swatch ink-mono"
                      data-selected={activePenColor === sw.code ? 'true' : undefined}
                      data-bordered={sw.needsBorder ? 'true' : undefined}
                      style:background={`var(--ink-pen-${sw.code})`}
                      style:color={sw.lightFg ? 'var(--ink-pen-fg-light)' : 'var(--ink-pen-fg-dark)'}
                      aria-hidden="true"
                    >
                      {sw.code}
                    </span>
                  {/if}
                {/each}
              </span>
            {:else}
              <span
                class="ink-island-pen-dot"
                data-bordered={activePenColor === 'K' ? 'true' : undefined}
                style:background={`var(--ink-pen-${activePenColor})`}
                aria-hidden="true"
              ></span>
            {/if}
          </span>
        {/if}
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

      {#if pathItems.length > 0}
        <span class="ink-island-path">
          {#each pathItems as seg, i (i)}
            {#if i > 0}
              <span class="ink-island-sep" aria-hidden="true">›</span>
            {/if}
            <span class="ink-island-seg" data-active={seg.active ? 'true' : undefined}>{seg.label}</span>
          {/each}
        </span>
      {/if}

      {#if penWidth}
        <span class="ink-island-val ink-mono" data-muted="true">{penWidth}</span>
      {/if}

      {#if detail}
        {#if boardMeta?.dotBeforeDetail}
          <span class="ink-island-middot" aria-hidden="true">·</span>
        {/if}
        <span class="ink-island-detail">{detail}</span>
      {/if}

      {#if value}
        {#if boardMeta?.dotBeforeValue}
          <span class="ink-island-middot" aria-hidden="true">·</span>
        {/if}
        <span class="ink-island-val ink-mono" data-muted={valueMuted ? 'true' : undefined}>{value}</span>
      {/if}

      {#if Array.isArray(tags) && tags.length > 0}
        {#each tags as tag (tag)}
          <span class="ink-island-tag">{tag}</span>
        {/each}
      {/if}

      {#if variant === 'record' && micOff}
        <span class="ink-island-icon" data-tone="sub">
          <Icon name="mic-off" size={16} label={t('islandUi.record.micOff')} />
        </span>
      {/if}

      {#if combo || (keys && keys.length)}
        <Keycap {combo} {keys} {platform} variant="island" />
      {/if}

      {#if keyHint}
        <span class="ink-island-detail">{keyHint}</span>
      {/if}
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
        {#if button.key && !omitButtonKeys}
          <kbd class="ink-island-btn-key ink-mono">{formatButtonKey(button.key)}</kbd>
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
        {#if secondaryButton.key && !omitButtonKeys}
          <kbd class="ink-island-btn-key ink-mono">{formatButtonKey(secondaryButton.key)}</kbd>
        {/if}
      </button>
    {/if}

    {#if chip}
      <button
        type="button"
        class="ink-island-btn ink-island-chip ink-focus"
        onclick={chip.onClick}
      >
        {#if chipHex}
          <span
            class="ink-island-chip-swatch"
            style:background={chipHex}
            aria-hidden="true"
          ></span>
        {/if}
        <span class="ink-island-chip-label ink-mono">{chip.label}</span>
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
    transition:
      opacity 300ms ease,
      transform 180ms cubic-bezier(0.2, 0, 0, 1);
  }

  .ink-island[data-idle-dim='true'] {
    opacity: 0.4;
  }

  .ink-island[data-avoid='true'] {
    transform: translateY(-120%);
    opacity: 0.15;
    pointer-events: none;
  }

  .ink-island[data-twoline='true'] {
    border-radius: 22px;
    padding: 10px 16px;
  }

  .ink-island[data-thumb='true'] {
    padding: 5px 6px 5px 5px;
  }

  .ink-island[data-result-card='true'] {
    border-radius: 22px;
    padding: 7px 9px 10px 12px;
  }

  .ink-island[data-result-card='true'][data-thumb='true'] {
    padding: 7px 8px 10px 8px;
  }

  .ink-island[data-muted='true'] {
    background: var(--ink-island-muted);
    color: var(--ink-island-text-2);
  }

  .ink-island-row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 38px;
    min-width: 0;
    max-width: 100%;
  }

  .ink-island[data-twoline='true'] .ink-island-row {
    min-height: 20px;
  }

  .ink-island[data-thumb='true'] .ink-island-row {
    min-height: 30px;
  }

  .ink-island[data-result-card='true'] .ink-island-row {
    min-height: 36px;
    gap: 6px;
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

  .ink-island[data-result-card='true'] .ink-island-thumb {
    width: 56px;
    height: 36px;
    border-radius: 15px 7px 7px 15px;
  }

  .ink-island-thumb-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .ink-island-result-body {
    flex: 1 1 auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    overflow: hidden;
  }

  .ink-island-result-title {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    font-size: 13px;
    color: var(--ink-island-text);
  }

  .ink-island-result-title strong {
    font-weight: 600;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ink-island-result-meta {
    font-size: 11px;
    line-height: 1.3;
    color: var(--ink-island-sub);
    min-width: 0;
    white-space: normal;
    word-break: keep-all;
    overflow-wrap: break-word;
  }

  .ink-island-path {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--ink-island-sub);
  }

  .ink-island-sep {
    color: var(--ink-island-sub);
  }

  .ink-island-seg[data-active='true'] {
    color: var(--ink-island-text);
    font-weight: 500;
  }

  .ink-island-tag {
    display: inline-flex;
    align-items: center;
    height: 18px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--ink-island-key-bg);
    color: var(--ink-island-text-2);
    font-size: 11px;
    font-weight: 500;
    flex: none;
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

  .ink-island-palette {
    display: inline-flex;
    align-items: center;
    flex: none;
  }

  .ink-island-swatches {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 3px;
  }

  .ink-island-swatch {
    width: 16px;
    height: 16px;
    border: 0;
    border-radius: 50%;
    padding: 0;
    display: inline-grid;
    place-items: center;
    font-size: 9px;
    font-weight: 600;
    line-height: 1;
    flex: none;
    box-sizing: border-box;
  }

  button.ink-island-swatch {
    cursor: pointer;
  }

  .ink-island-swatch[data-bordered='true'] {
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.35);
  }

  .ink-island-swatch[data-selected='true'] {
    box-shadow:
      0 0 0 1.5px var(--ink-island),
      0 0 0 3px var(--ink-island-text);
  }

  .ink-island-pen-dot {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    box-shadow: 0 0 0 2px var(--ink-island), 0 0 0 3.5px var(--ink-island-text);
    flex: none;
  }

  .ink-island-pen-dot[data-bordered='true'] {
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.4),
      0 0 0 2px var(--ink-island),
      0 0 0 3.5px var(--ink-island-text);
  }

  .ink-island-middot {
    color: var(--ink-island-sub);
    font-size: 12px;
    margin: 0 -4px;
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

  .ink-island-chip {
    background: var(--ink-island-key-bg);
    color: var(--ink-island-text);
    box-shadow: var(--ink-island-ring);
    padding: 0 10px 0 8px;
    gap: 6px;
  }

  .ink-island[data-result-card='true'] .ink-island-btn {
    padding: 0 9px;
    gap: 5px;
  }

  .ink-island[data-result-card='true'] .ink-island-chip {
    padding: 0 9px 0 7px;
    gap: 5px;
  }

  .ink-island-chip-swatch {
    width: 10px;
    height: 10px;
    border-radius: 3px;
    border: 1px solid rgba(255, 255, 255, 0.8);
    box-sizing: border-box;
    flex: none;
  }

  .ink-island-chip-label {
    font-size: 11.5px;
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
