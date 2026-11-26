<script>
  import { t } from '../i18n/index.svelte.js';

  /**
   * 개발자 표기(docs/brand/README.md 2장) — Tidy Task·롤린썬더와 같은 찰떡쌤 마스코트 + 이름.
   * `copyright` = `© 2026 [마스코트] 찰떡쌤. MIT License.` / `maker` = `[마스코트] Made by. 찰떡쌤`
   * @type {{ variant?: 'copyright' | 'maker', class?: string }}
   */
  let { variant = 'copyright', class: className = '' } = $props();

  const MASCOT_1X = new URL('../icons/chaltteok@1x.webp', import.meta.url).href;
  const MASCOT_2X = new URL('../icons/chaltteok@2x.webp', import.meta.url).href;
</script>

{#snippet mascot()}
  <img
    class="ink-credit-mascot"
    src={MASCOT_1X}
    srcset="{MASCOT_1X} 1x, {MASCOT_2X} 2x"
    width="16"
    height="16"
    alt=""
    aria-hidden="true"
    draggable="false"
  />
{/snippet}

<p class="ink-credit {className}" data-variant={variant}>
  {#if variant === 'maker'}
    {@render mascot()}<span>{t('common.credit.madeBy')}</span>
  {:else}
    <span>© 2026</span>{@render mascot()}<span>{t('common.credit.copyrightHolder')}</span>
  {/if}
</p>

<style>
  .ink-credit {
    margin: 0;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    line-height: 16px;
    color: var(--ink-sub);
    white-space: nowrap;
    user-select: none;
  }

  .ink-credit-mascot {
    flex: none;
    width: 16px;
    height: 16px;
    object-fit: contain;
    -webkit-user-drag: none;
  }
</style>
