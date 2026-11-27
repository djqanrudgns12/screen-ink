<script>
  /**
   * 녹화 범위 표시(04 S11 · 단계 6.4): 녹화하는 모니터를 덮는 투명·클릭 통과 창 안에
   * 네 귀 빨간 꺾쇠 + 바깥 22% 어둡게. 창은 캡처 제외라 영상에는 안 찍힌다(R-REC-2).
   * 좌표는 Rust가 정한 창 안 논리 px 그대로 쓴다(바꾸지 않는다).
   * @type {{ x: number, y: number, width: number, height: number, aligned?: boolean }}
   */
  import SelectionFrame from '../components/SelectionFrame.svelte';
  import { t } from '../i18n/index.svelte.js';

  let { x, y, width, height, aligned = false } = $props();
</script>

<div class="ink-rec-frame">
  <SelectionFrame {x} {y} {width} {height} {aligned} state="recording" scrim={true} instant={true} label={t('recordUi.frame.aria')} />
</div>

<style>
  .ink-rec-frame {
    position: fixed;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
  }
</style>
