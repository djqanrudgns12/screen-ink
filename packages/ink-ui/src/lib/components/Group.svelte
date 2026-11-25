<script>
  /**
   * 1단계 묶음 상자(카드 안 카드 금지, 안쪽은 머리카락 선으로만 나눔).
   * @type {{ children?: import('svelte').Snippet, class?: string }}
   */
  let { children, class: className = '' } = $props();
</script>

<div class="ink-group {className}">
  {@render children?.()}
</div>

<style>
  .ink-group {
    background: var(--ink-group-bg);
    border-radius: var(--ink-r-group);
    box-shadow: var(--ink-group-ring);
    overflow: hidden;
  }
</style>
