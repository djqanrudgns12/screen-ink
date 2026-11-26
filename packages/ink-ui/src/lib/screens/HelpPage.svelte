<script>
  import Icon from '../icons/Icon.svelte';
  import Keycap from '../components/Keycap.svelte';
  import HelpGuideModal from '../components/help/HelpGuideModal.svelte';
  import { HELP_GUIDES } from '../components/help/guides.js';
  import { t } from '../i18n/index.svelte.js';

  /**
   * 설정 › 사용 설명서 탭(R2-HELP-1~4).
   * 카드 8장(찍기 3종 / 표시·고정·복사 / 그리기 / 칠판 / 확대 / 녹화 / 단축키 / 문제 해결).
   * @type {{
   *   client: import('@screen-ink/client').InkClient,
   *   onNavigateTab?: (tab: 'shortcuts' | 'general') => void,
   *   onOpenCheckup?: (target: 'checkup' | 'reset') => void,
   *   initialGuideId?: string | null,
   * }}
   */
  let {
    client,
    onNavigateTab = undefined,
    onOpenCheckup = undefined,
    initialGuideId = null,
  } = $props();

  let state = $state(null);
  let activeGuideId = $state(/** @type {string | null} */ (null));
  let modalOpen = $state(false);

  $effect(() => {
    if (initialGuideId) {
      activeGuideId = initialGuideId;
      modalOpen = true;
    }
  });

  $effect(() => {
    let alive = true;
    client.getState().then((s) => {
      if (alive) state = s;
    });
    const off = client.subscribe((e) => {
      if (e.type === 'state') state = e.state;
    });
    return () => {
      alive = false;
      off();
    };
  });

  let platform = $derived(state?.platform ?? 'windows');
  let shortcuts = $derived(state?.shortcuts ?? []);
  let activeGuide = $derived(HELP_GUIDES.find((g) => g.id === activeGuideId) ?? null);

  function comboFor(shortcutId) {
    if (!shortcutId) return '';
    return shortcuts.find((s) => s.id === shortcutId)?.combo ?? '';
  }

  function openGuide(guide) {
    activeGuideId = guide.id;
    modalOpen = true;
  }

  function handleTry(guide) {
    if (guide.id === 'troubleshoot') {
      onOpenCheckup?.('checkup');
      return;
    }
    if (guide.targetTab) {
      onNavigateTab?.(guide.targetTab);
      return;
    }
    if (guide.tryAction) {
      client.run(/** @type {any} */ (guide.tryAction));
    }
  }
</script>

<div class="ink-page">
  <header class="ink-page-head" data-tauri-drag-region>
    <h1 class="ink-page-title" data-tauri-drag-region>{t('help.title')}</h1>
    <p class="ink-page-sub" data-tauri-drag-region>{t('help.subtitle')}</p>
  </header>

  <div class="ink-page-body ink-scroll">
    <div class="ink-help-grid">
      {#each HELP_GUIDES as guide (guide.id)}
        {@const combo = comboFor(guide.shortcutId)}
        <button
          type="button"
          class="ink-help-card ink-focus"
          data-setting-id={`help:${guide.id}`}
          onclick={() => openGuide(guide)}
        >
          <div class="ink-help-card-top">
            <span class="ink-help-card-icon">
              <Icon name={guide.icon} size={18} />
            </span>
            {#if combo}
              <Keycap {combo} {platform} />
            {:else if guide.shortcutId}
              <span class="ink-help-unset">{t('help.card.unset')}</span>
            {/if}
          </div>
          <h2 class="ink-help-card-title">{t(`help.${guide.id}.title`)}</h2>
          <p class="ink-help-card-desc">{t(`help.${guide.id}.summary`)}</p>
        </button>
      {/each}
    </div>
  </div>
</div>

<HelpGuideModal
  bind:open={modalOpen}
  guide={activeGuide}
  combo={activeGuide ? comboFor(activeGuide.shortcutId) : ''}
  {platform}
  onTry={handleTry}
  onCheckup={(target) => onOpenCheckup?.(target)}
/>

<style>
  .ink-page {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .ink-page-head {
    flex: none;
    padding: 18px 60px 10px 26px;
  }

  .ink-page-title {
    margin: 0;
    font-family: var(--ink-font-display);
    font-size: 19px;
    font-weight: 600;
    color: var(--ink-text);
    white-space: nowrap;
    user-select: none;
  }

  .ink-page-sub {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--ink-sub);
  }

  .ink-page-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
    padding: 4px 20px 22px 26px;
  }

  .ink-help-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .ink-help-card {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
    padding: 13px 14px;
    border: 0;
    border-radius: var(--ink-r-group);
    background: var(--ink-group-bg);
    box-shadow: var(--ink-group-ring);
    color: var(--ink-text);
    font-family: inherit;
    text-align: left;
    cursor: pointer;
    transition: background-color 120ms ease;
  }

  .ink-help-card:hover {
    background: var(--ink-hover);
  }

  .ink-help-card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    min-height: 26px;
  }

  .ink-help-card-icon {
    width: 28px;
    height: 28px;
    border-radius: var(--ink-r-row);
    background: var(--ink-raised);
    color: var(--ink-text);
    display: grid;
    place-items: center;
    flex: none;
  }

  .ink-help-unset {
    font-size: 11px;
    color: var(--ink-sub);
  }

  .ink-help-card-title {
    margin: 2px 0 0;
    font-size: 13.5px;
    font-weight: 600;
    line-height: 1.3;
    color: var(--ink-text);
  }

  .ink-help-card-desc {
    margin: 0;
    font-size: 12px;
    line-height: 1.45;
    color: var(--ink-sub);
  }
</style>
