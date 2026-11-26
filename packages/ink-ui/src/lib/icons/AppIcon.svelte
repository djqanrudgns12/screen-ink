<script>
  /**
   * 공식 앱 아이콘(슉쇽, docs/brand/README.md): 먹색 6날 조리개 + 날개 사이 흰 음각 펜촉 + 주황 숨구멍 점.
   * 원형, 원 바깥은 투명. 원본은 docs/brand/official/Shookshok-icon-1024.png, 여기 PNG는 `npx tauri icon`이 만든 축소본.
   * 보이는 크기의 2배 이상 원본을 골라 고밀도 화면에서도 흐리지 않게 한다.
   * @type {{ size?: 16 | 24 | 28 | 32 | 56 | 64, label?: string, class?: string }}
   */
  let { size = 24, label = '', class: className = '' } = $props();

  const SRC = {
    32: new URL('./app-icon-32.png', import.meta.url).href,
    64: new URL('./app-icon-64.png', import.meta.url).href,
    128: new URL('./app-icon-128.png', import.meta.url).href,
  };
  const src = $derived(size <= 16 ? SRC[32] : size <= 32 ? SRC[64] : SRC[128]);
</script>

<img
  {src}
  width={size}
  height={size}
  alt={label}
  aria-hidden={label ? undefined : 'true'}
  draggable="false"
  class="ink-app-icon {className}"
/>

<style>
  .ink-app-icon {
    flex: none;
    display: inline-block;
    user-select: none;
    -webkit-user-drag: none;
  }
</style>
