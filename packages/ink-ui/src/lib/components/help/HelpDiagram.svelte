<script>
  import Icon from '../../icons/Icon.svelte';
  import { t } from '../../i18n/index.svelte.js';

  /**
   * 사용 설명서 모달 안의 아이콘 순서 도식(R2-HELP-2).
   * 실제 화면 캡처 없이 맞춤 아이콘만으로 3단계 흐름을 보여 준다.
   * @type {{ nodes: Array<{ icon: string, labelKey: string }> }}
   */
  let { nodes = [] } = $props();
</script>

<div class="ink-help-diagram" role="img" aria-label={t('help.modal.diagramAria')}>
  {#each nodes as node, i (node.labelKey)}
    {#if i > 0}
      <span class="ink-help-arrow" aria-hidden="true">→</span>
    {/if}
    <div class="ink-help-node">
      <span class="ink-help-node-icon">
        <Icon name={node.icon} size={20} />
      </span>
      <span class="ink-help-node-label">{t(node.labelKey)}</span>
    </div>
  {/each}
</div>

<style>
  .ink-help-diagram {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 12px 14px;
    border-radius: var(--ink-r-group);
    background: var(--ink-group-bg);
    box-shadow: var(--ink-group-ring);
  }

  .ink-help-node {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    text-align: center;
  }

  .ink-help-node-icon {
    width: 36px;
    height: 36px;
    border-radius: var(--ink-r-row);
    background: var(--ink-raised);
    color: var(--ink-text);
    display: grid;
    place-items: center;
  }

  .ink-help-node-label {
    font-size: 11.5px;
    line-height: 1.3;
    color: var(--ink-text-2);
  }

  .ink-help-arrow {
    color: var(--ink-muted);
    font-size: 14px;
    flex: none;
    margin-bottom: 18px;
  }
</style>
