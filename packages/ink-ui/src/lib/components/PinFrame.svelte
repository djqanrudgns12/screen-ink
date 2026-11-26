<script>
  import { t } from '../i18n/index.svelte.js';

  /**
   * 창 맨 위에 고정 테두리(F16 · 10 §5).
   * 제목 막대 단추와 겹치지 않도록 배지 없이 주황 테두리만 그린다.
   * @type {{
   *   borderWidth?: 0 | 2 | 4 | 6,
   *   radius?: number,
   *   active?: boolean,
   *   children?: import('svelte').Snippet,
   *   class?: string,
   * }}
   */
  let {
    borderWidth = 4,
    radius = 10,
    active = true,
    children = undefined,
    class: className = '',
  } = $props();
</script>

<div
  aria-label={t('pin.topmost.frameAria')}
  class="ink-pin-frame {className}"
  data-active={active && borderWidth > 0 ? 'true' : undefined}
  style:border-radius={`${radius}px`}
  style:--pin-w={`${active ? borderWidth : 0}px`}
>
  {@render children?.()}
</div>

<style>
  .ink-pin-frame {
    position: relative;
    overflow: hidden;
    transition: box-shadow 140ms cubic-bezier(0.2, 1.15, 0.4, 1);
  }

  .ink-pin-frame[data-active='true'] {
    box-shadow: 0 0 0 var(--pin-w, 4px) var(--ink-accent), var(--ink-shadow-pin);
  }
</style>
