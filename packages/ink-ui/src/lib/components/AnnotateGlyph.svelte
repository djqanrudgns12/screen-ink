<script>
  import Icon from '../icons/Icon.svelte';
  import { ICONS } from '../icons/icons.js';

  /**
   * 캡처·판서 툴바 글리프(R2-ICON). 슉쇽 맞춤 세트(`icons.js`)를 직접 사용한다.
   * `filled`가 참이고 `*-fill` 변형이 있으면 채움 변형을 그린다(지금 고른 도구 표시).
   * @type {{ name: string, size?: 16 | 18 | 20 | 24, filled?: boolean, label?: string, class?: string }}
   */
  let { name, size = 18, filled = false, label = '', class: className = '' } = $props();

  let resolved = $derived(filled && ICONS[`${name}-fill`] ? `${name}-fill` : name);
</script>

<Icon name={resolved} {size} accent={false} {label} class={className} />
