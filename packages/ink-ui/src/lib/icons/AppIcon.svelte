<script>
  /**
   * screen-ink 앱 아이콘(F01 벡터 마스터) — 짙은 흑연 squircle + 흰 조준틀 + 주황 잉크 방울.
   * @type {{ size?: 16 | 24 | 28 | 32 | 56 | 64, label?: string, class?: string }}
   */
  let { size = 24, label = '', class: className = '' } = $props();
  const uid = `ink-app-${Math.random().toString(36).slice(2, 8)}`;
</script>

{#if size <= 16}
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    role={label ? 'img' : undefined}
    aria-label={label || undefined}
    aria-hidden={label ? undefined : 'true'}
    class="ink-app-icon {className}"
  >
    <rect width="16" height="16" rx="3.6" fill="var(--ink-brand-top)" />
    <path
      d="M3.5 6.5v-2a1 1 0 0 1 1-1h2M9.5 3.5h2a1 1 0 0 1 1 1v2M12.5 9.5v2a1 1 0 0 1-1 1h-2M6.5 12.5h-2a1 1 0 0 1-1-1v-2"
      fill="none"
      stroke="var(--ink-brand-frame)"
      stroke-width="1.3"
      stroke-linecap="round"
    />
    <circle cx="8" cy="8.4" r="2" fill="var(--ink-accent)" />
  </svg>
{:else}
  <svg
    width={size}
    height={size}
    viewBox="0 0 64 64"
    role={label ? 'img' : undefined}
    aria-label={label || undefined}
    aria-hidden={label ? undefined : 'true'}
    class="ink-app-icon {className}"
  >
    <defs>
      <linearGradient id={uid} x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="var(--ink-brand-top)" />
        <stop offset="1" stop-color="var(--ink-brand-bottom)" />
      </linearGradient>
    </defs>
    <rect width="64" height="64" rx="14.5" fill="url(#{uid})" />
    <rect x="0.5" y="0.5" width="63" height="63" rx="14" fill="none" stroke="var(--ink-brand-rim)" />
    <path
      d="M17 25v-5a3 3 0 0 1 3-3h5M39 17h5a3 3 0 0 1 3 3v5M47 39v5a3 3 0 0 1-3 3h-5M25 47h-5a3 3 0 0 1-3-3v-5"
      fill="none"
      stroke="var(--ink-brand-frame)"
      stroke-width={size <= 32 ? 4 : 3.4}
      stroke-linecap="round"
    />
    <path
      d="M32 22.5c4.2 5.2 6.5 8.5 6.5 11.3a6.5 6.5 0 0 1-13 0c0-2.8 2.3-6.1 6.5-11.3z"
      fill="var(--ink-accent)"
    />
    {#if size >= 48}
      <ellipse cx="29.6" cy="33.4" rx="1.5" ry="2.5" fill="var(--ink-brand-highlight)" />
    {/if}
  </svg>
{/if}

<style>
  .ink-app-icon {
    flex: none;
    display: inline-block;
  }
</style>
