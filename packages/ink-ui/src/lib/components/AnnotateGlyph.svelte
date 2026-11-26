<script>
  import Icon from '../icons/Icon.svelte';
  import { ICONS } from '../icons/icons.js';

  /**
   * 캡처 주석 툴바 글리프(R-CAP-28). 아이콘 세트(icons.js)에 있으면 그것을, 없으면 같은 규칙(24 격자·선 1.5·둥근 끝)의
   * 임시 선 그림을 쓴다 — 확정 아이콘은 브랜드 세션이 icons.js에 더하면 저절로 바뀐다.
   * @type {{ name: string, size?: 20 | 24 }}
   */
  let { name, size = 20 } = $props();

  /** 임시 선 그림(path d 목록). @type {Record<string, string[]>} */
  const FALLBACK = {
    rect: ['M4.5 6.5h15v11h-15z'],
    ellipse: ['M12 5.5c4.7 0 8.5 2.9 8.5 6.5s-3.8 6.5-8.5 6.5-8.5-2.9-8.5-6.5 3.8-6.5 8.5-6.5z'],
    arrow: ['M5 19L18.5 5.5', 'M10.5 5.5h8v8'],
    pen: ['M4.5 19.5l1-4.5L16 4.5l3.5 3.5L9 18.5z', 'M13.5 7l3.5 3.5'],
    marker: ['M14.5 4.5l5 5-7.5 7.5H7v-5z', 'M4 20h8'],
    mosaic: ['M4.5 4.5h15v15h-15z', 'M4.5 9.5h15M4.5 14.5h15M9.5 4.5v15M14.5 4.5v15'],
    text: ['M6 6h12', 'M12 6v13', 'M9.5 19h5'],
    redo: ['M15 6.5l4.5 4.5-4.5 4.5', 'M19 11h-9a5 5 0 0 0 0 10h3'],
  };

  let inSet = $derived(Boolean(ICONS[name]));
  let paths = $derived(FALLBACK[name] ?? []);
</script>

{#if inSet}
  <Icon {name} {size} accent={false} />
{:else}
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    class="ink-annot-glyph"
  >
    {#each paths as d, i (i)}
      <path {d} />
    {/each}
    {#if name === 'mosaic'}
      <path d="M4.5 4.5h5v5h-5zM14.5 9.5h5v5h-5zM9.5 14.5h5v5h-5z" fill="currentColor" stroke="none" opacity="0.55" />
    {/if}
  </svg>
{/if}

<style>
  .ink-annot-glyph {
    flex: none;
    display: inline-block;
    vertical-align: middle;
  }
</style>
