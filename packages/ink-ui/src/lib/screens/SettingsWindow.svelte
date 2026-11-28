<script>
  import { withSlotLabels } from '../models/shortcutLabels.js';
  import { tick } from 'svelte';
  import AppIcon from '../icons/AppIcon.svelte';
  import Icon from '../icons/Icon.svelte';
  import Keycap from '../components/Keycap.svelte';
  import Banner from '../components/Banner.svelte';
  import SectionLabel from '../components/SectionLabel.svelte';
  import Group from '../components/Group.svelte';
  import Row from '../components/Row.svelte';
  import MakerCredit from '../components/MakerCredit.svelte';
  import ShortcutsPage from './ShortcutsPage.svelte';
  import CapturePage from './CapturePage.svelte';
  import DrawZoomPage from './DrawZoomPage.svelte';
  import GeneralPage from './GeneralPage.svelte';
  import HelpPage from './HelpPage.svelte';
  import FirstRunSheet from './FirstRunSheet.svelte';
  import { searchSettings } from '../models/settingsSearch.js';
  import { syncLang, t } from '../i18n/index.svelte.js';

  /**
   * 설정 창(S02~S05 + 사용 설명서 + 설정 찾기, 900×600) + S06 첫 실행 카드.
   * 설정 창 안 Ctrl+Z(맥 Cmd+Z) = client.undo(), Ctrl+K(맥 Cmd+K) = 설정 찾기.
   * @type {{
   *   client: import('@screen-ink/client').InkClient,
   *   initialTab?: 'shortcuts' | 'capture' | 'draw' | 'general' | 'help' | 'info',
   *   firstRun?: boolean,
   *   initialHelpGuide?: string | null,
   *   onClose?: () => void,
   *   onFirstRunClose?: () => void,
   *   class?: string,
   * }}
   */
  let {
    client,
    initialTab = 'shortcuts',
    firstRun = false,
    initialHelpGuide = null,
    onClose = undefined,
    onFirstRunClose = undefined,
    class: className = '',
  } = $props();

  const NAV_ITEMS = [
    { id: 'shortcuts', key: 'settings.tab.shortcuts', icon: 'shortcuts' },
    { id: 'capture', key: 'settings.tab.capture', icon: 'region' },
    { id: 'draw', key: 'settings.tab.draw', icon: 'draw' },
    { id: 'general', key: 'settings.tab.general', icon: 'settings' },
    { id: 'help', key: 'settings.tab.help', icon: 'help-circle' },
  ];

  let activeTab = $state('shortcuts');
  let state = $state(null);
  let isRecordingShortcut = $state(false);
  let firstRunOpen = $state(false);
  let noticesDismissed = $state(false);

  // 설정 찾기(R2-SET-1)
  let searchQuery = $state('');
  let searchOpen = $state(false);
  let searchIndex = $state(0);
  let helpInitialGuide = $state(/** @type {string | null} */ (null));
  /** @type {HTMLInputElement | null} */
  let searchInputEl = $state(null);
  /** @type {HTMLElement | null} */
  let mainEl = $state(null);
  let flashTimer = /** @type {ReturnType<typeof setTimeout> | null} */ (null);
  let lastFlashedEl = /** @type {HTMLElement | null} */ (null);

  $effect(() => {
    const on = isRecordingShortcut;
    void client?.shortcutCapture?.(on);
  });
  $effect(() => () => {
    void client?.shortcutCapture?.(false);
    if (flashTimer) clearTimeout(flashTimer);
  });

  $effect(() => {
    activeTab = initialTab;
  });

  $effect(() => {
    helpInitialGuide = initialHelpGuide;
  });

  $effect(() => {
    firstRunOpen = firstRun;
  });

  $effect(() => {
    let alive = true;
    client.getState().then((s) => {
      if (alive) {
        state = s;
        syncLang(s);
      }
    });
    const off = client.subscribe((e) => {
      if (e.type === 'state') {
        state = e.state;
        syncLang(e.state);
      }
    });
    return () => {
      alive = false;
      off();
    };
  });

  let platform = $derived(state?.platform ?? 'windows');
  let lang = $derived(state?.settings?.['general.lang'] ?? 'ko');
  let searchResults = $derived(
    searchSettings(searchQuery, {
      lang,
      platform,
      shortcuts: withSlotLabels(state?.shortcuts ?? [], t),
      translate: t,
      limit: 8,
    }),
  );

  $effect(() => {
    if (searchIndex >= searchResults.length) {
      searchIndex = 0;
    }
  });

  let noticeText = $derived(
    !noticesDismissed && Array.isArray(state?.notices) && state.notices.length > 0
      ? state.notices.join(' · ')
      : '',
  );

  function switchTab(tabId) {
    activeTab = tabId;
    isRecordingShortcut = false;
    if (tabId !== 'help') helpInitialGuide = null;
  }

  async function jumpToSetting(tabId, settingId, matchTexts = []) {
    switchTab(tabId);
    searchOpen = false;
    if (settingId?.startsWith('help:')) {
      helpInitialGuide = settingId.slice('help:'.length);
    }

    await tick();
    if (!mainEl) return;

    const findTarget = () => {
      if (!mainEl) return null;
      if (settingId) {
        const direct = /** @type {HTMLElement | null} */ (
          mainEl.querySelector(`[data-setting-id="${settingId}"]`)
        );
        if (direct) return direct;
      }
      if (matchTexts.length > 0) {
        const needles = matchTexts.map((m) => m.toLowerCase());
        const rows = mainEl.querySelectorAll('.ink-row-wrap, .ink-help-card');
        for (const el of rows) {
          const label = el.querySelector('.ink-row-label, .ink-help-card-title')?.textContent?.toLowerCase() ?? '';
          if (needles.some((n) => label.includes(n))) {
            return /** @type {HTMLElement} */ (el);
          }
        }
      }
      return null;
    };

    const highlight = () => {
      const target = findTarget();
      if (!target) return;
      target.scrollIntoView({ block: 'center', behavior: 'auto' });
      if (lastFlashedEl) lastFlashedEl.classList.remove('ink-search-flash');
      if (flashTimer) clearTimeout(flashTimer);
      target.classList.remove('ink-search-flash');
      // 리플로우 후 클래스를 붙여 반복 선택에도 반짝임이 재생된다
      void target.offsetWidth;
      target.classList.add('ink-search-flash');
      lastFlashedEl = target;
      flashTimer = setTimeout(() => {
        target.classList.remove('ink-search-flash');
      }, 1400);
    };

    if (typeof requestAnimationFrame === 'function') {
      requestAnimationFrame(highlight);
    } else {
      highlight();
    }
  }

  function handleSearchKeydown(e) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (searchResults.length > 0) {
        searchOpen = true;
        searchIndex = (searchIndex + 1) % searchResults.length;
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (searchResults.length > 0) {
        searchOpen = true;
        searchIndex = (searchIndex - 1 + searchResults.length) % searchResults.length;
      }
    } else if (e.key === 'Enter' && searchResults.length > 0) {
      e.preventDefault();
      const picked = searchResults[searchIndex] ?? searchResults[0];
      if (picked) void jumpToSetting(picked.tab, picked.id, picked.matchTexts);
    } else if (e.key === 'Escape') {
      e.stopPropagation();
      if (searchQuery) {
        searchQuery = '';
      } else {
        searchOpen = false;
        searchInputEl?.blur();
      }
    }
  }

  function handleWindowKeydown(e) {
    if (isRecordingShortcut || firstRunOpen) return;
    if (typeof document !== 'undefined' && document.querySelector('[data-dialog-content]')) {
      return;
    }

    const tag = e.target?.tagName;
    const isEditable = tag === 'INPUT' || tag === 'TEXTAREA' || e.target?.isContentEditable;
    const isMac = state?.platform === 'macos';
    const primaryMod = isMac ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey;

    if (primaryMod && !e.altKey && !e.shiftKey && (e.code === 'KeyK' || e.key.toLowerCase() === 'k')) {
      e.preventDefault();
      searchInputEl?.focus();
      searchInputEl?.select();
      searchOpen = true;
      return;
    }

    if (primaryMod && !e.altKey && !e.shiftKey && (e.code === 'KeyZ' || e.key.toLowerCase() === 'z')) {
      if (!isEditable) {
        e.preventDefault();
        client.undo();
      }
      return;
    }

    if (e.key === 'Escape' && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) {
      if (searchOpen) {
        searchOpen = false;
        return;
      }
      onClose?.();
    }
  }

  function handleFirstRunClose() {
    firstRunOpen = false;
    onFirstRunClose?.();
  }

  function handleFirstRunShortcuts() {
    firstRunOpen = false;
    switchTab('shortcuts');
    onFirstRunClose?.();
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<div role="dialog" aria-label={t('settings.title')} class="ink-settings-win {className}">
  <nav aria-label={t('settings.nav.aria')} class="ink-sidebar">
    <div class="ink-brand" data-tauri-drag-region>
      <span class="ink-brand-icon" data-tauri-drag-region>
        <AppIcon size={28} />
      </span>
      <div class="ink-brand-meta" data-tauri-drag-region>
        <p class="ink-brand-name" data-tauri-drag-region>{t('settings.brand')}</p>
        <p class="ink-brand-ver ink-mono" data-tauri-drag-region>{state?.version ?? '0.1.0'}</p>
      </div>
    </div>

    <div class="ink-search-box">
      <div class="ink-search-field">
        <span class="ink-search-icon" aria-hidden="true">
          <Icon name="zoom" size={13} />
        </span>
        <input
          bind:this={searchInputEl}
          type="text"
          class="ink-search-input ink-focus"
          placeholder={t('settings.search.placeholder')}
          aria-label={t('settings.search.aria')}
          bind:value={searchQuery}
          onfocus={() => (searchOpen = true)}
          oninput={() => {
            searchOpen = true;
            searchIndex = 0;
          }}
          onkeydown={handleSearchKeydown}
        />
        {#if searchQuery}
          <button
            type="button"
            class="ink-search-clear ink-focus"
            aria-label={t('settings.search.clear')}
            onclick={() => {
              searchQuery = '';
              searchInputEl?.focus();
            }}
          >
            <Icon name="close" size={11} />
          </button>
        {:else}
          <span class="ink-search-kbd ink-mono" aria-hidden="true">{t('settings.search.shortcut')}</span>
        {/if}
      </div>

      {#if searchOpen && searchQuery.trim().length > 0}
        <div class="ink-search-popover ink-scroll" role="listbox" aria-label={t('settings.search.resultsAria')}>
          {#if searchResults.length === 0}
            <p class="ink-search-empty">{t('settings.search.empty')}</p>
          {:else}
            {#each searchResults as item, idx (item.id)}
              <button
                type="button"
                role="option"
                aria-selected={idx === searchIndex}
                class="ink-search-item ink-focus"
                data-active={idx === searchIndex ? 'true' : undefined}
                onmouseenter={() => (searchIndex = idx)}
                onclick={() => jumpToSetting(item.tab, item.id, item.matchTexts)}
              >
                <span class="ink-search-item-main">
                  <span class="ink-search-item-title">{item.title}</span>
                  <span class="ink-search-item-tab">{item.tabLabel}</span>
                </span>
                {#if item.combo}
                  <Keycap combo={item.combo} {platform} variant="dim" />
                {/if}
              </button>
            {/each}
          {/if}
        </div>
      {/if}
    </div>

    {#each NAV_ITEMS as item (item.id)}
      {@const on = activeTab === item.id}
      <button
        type="button"
        class="ink-nav ink-focus"
        data-active={on ? 'true' : undefined}
        aria-current={on ? 'page' : undefined}
        onclick={() => switchTab(item.id)}
      >
        <Icon name={item.icon} size={16} accent={on} inverted={on} />
        <span>{t(item.key)}</span>
      </button>
    {/each}

    <div class="ink-nav-spacer" data-tauri-drag-region></div>

    <button
      type="button"
      class="ink-nav ink-focus"
      data-active={activeTab === 'info' ? 'true' : undefined}
      aria-current={activeTab === 'info' ? 'page' : undefined}
      onclick={() => switchTab('info')}
    >
      <Icon name="info" size={16} accent={false} inverted={activeTab === 'info'} />
      <span>{t('settings.tab.info')}</span>
    </button>
  </nav>

  <main bind:this={mainEl} class="ink-main">
    <button
      type="button"
      aria-label={t('settings.close')}
      class="ink-close-btn ink-focus"
      onclick={() => onClose?.()}
    >
      <Icon name="close" size={16} />
    </button>

    {#if state?.readonly || noticeText}
      <div class="ink-top-banners">
        {#if state?.readonly}
          <Banner
            kind="conflict"
            message={t('settings.readonly')}
          />
        {/if}
        {#if noticeText}
          <Banner
            kind="info"
            message={noticeText}
            secondaryLabel={t('settings.close')}
            onSecondary={() => (noticesDismissed = true)}
          />
        {/if}
      </div>
    {/if}

    {#if activeTab === 'shortcuts'}
      <ShortcutsPage {client} onRecordingChange={(v) => (isRecordingShortcut = v)} />
    {:else if activeTab === 'capture'}
      <CapturePage {client} />
    {:else if activeTab === 'draw'}
      <DrawZoomPage {client} />
    {:else if activeTab === 'general'}
      <GeneralPage
        {client}
        onNavigateTab={(tabId) => switchTab(tabId)}
        onOpenFirstRun={() => {
          firstRunOpen = true;
          void client.run('firstRun.open');
        }}
      />
    {:else if activeTab === 'help'}
      <HelpPage
        {client}
        initialGuideId={helpInitialGuide}
        onNavigateTab={(tabId) => switchTab(tabId)}
        onOpenCheckup={(target) =>
          jumpToSetting('general', target === 'reset' ? 'general.reset' : 'general.checkup')
        }
      />
    {:else}
      <div class="ink-info-page">
        <header class="ink-info-head" data-tauri-drag-region>
          <h1 class="ink-info-title" data-tauri-drag-region>{t('settings.info.title')}</h1>
        </header>
        <div class="ink-info-body ink-scroll">
          <section data-setting-id="info.about">
            <SectionLabel title={t('settings.info.appSection')} />
            <Group>
              <Row label={t('settings.info.version')} sub={state?.version ?? '0.1.0'} subMono={true} />
              <Row label={t('settings.info.maker')} sub={t('settings.info.makerName')} />
              <Row label={t('settings.info.license')} sub="MIT" />
              <Row label={t('about.opensource.fontsLabel')} sub={t('about.opensource.fontsSub')} />
              <Row label={t('about.opensource.iconsLabel')} sub={t('about.opensource.iconsSub')} />
              <Row label={t('about.opensource.libsLabel')} sub={t('about.opensource.libsSub')} />
            </Group>
          </section>
          <section>
            <SectionLabel title={t('about.privacy.section')} />
            <Group>
              <Row label={t('about.privacy.label')} sub={t('about.privacy.sub')} />
            </Group>
          </section>
          <footer class="ink-info-credit">
            <p class="ink-info-credit-app">{t('settings.brand')} · v{state?.version ?? '0.1.0'}</p>
            <MakerCredit />
          </footer>
        </div>
      </div>
    {/if}
  </main>
</div>

{#if firstRunOpen}
  <FirstRunSheet
    {client}
    bind:open={firstRunOpen}
    onClose={handleFirstRunClose}
    onChangeShortcuts={handleFirstRunShortcuts}
  />
{/if}

<style>
  .ink-settings-win {
    position: relative;
    width: 900px;
    height: 600px;
    max-width: 100vw;
    max-height: 100vh;
    border-radius: var(--ink-r-window);
    overflow: hidden;
    display: flex;
    background: var(--ink-window-bg);
    color: var(--ink-text);
    box-shadow: var(--ink-shadow-window);
    font-family: var(--ink-font-sans);
  }

  .ink-sidebar {
    position: relative;
    width: 196px;
    flex: none;
    box-sizing: border-box;
    padding: 16px 10px;
    background: var(--ink-sidebar);
    border-right: 1px solid var(--ink-sidebar-border);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .ink-brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 10px 12px;
    user-select: none;
  }

  .ink-brand-icon {
    display: inline-flex;
    flex: none;
  }

  .ink-brand-meta {
    min-width: 0;
  }

  .ink-brand-name {
    margin: 0;
    font-family: var(--ink-font-display);
    font-size: 13px;
    font-weight: 600;
    color: var(--ink-text);
    white-space: nowrap;
  }

  .ink-brand-ver {
    margin: 0;
    font-size: 10.5px;
    color: var(--ink-sub);
  }

  .ink-search-box {
    position: relative;
    margin: 0 2px 10px;
  }

  .ink-search-field {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 29px;
    padding: 0 8px;
    border-radius: var(--ink-r-row);
    background: var(--ink-input-bg);
    box-shadow: inset 0 0 0 1px var(--ink-line);
  }

  .ink-search-field:focus-within {
    box-shadow: inset 0 0 0 1.5px var(--ink-accent);
  }

  .ink-search-icon {
    display: inline-flex;
    color: var(--ink-sub);
    flex: none;
  }

  .ink-search-input {
    flex: 1;
    min-width: 0;
    border: 0;
    padding: 0;
    background: transparent;
    color: var(--ink-text);
    font-family: inherit;
    font-size: 12px;
    outline: none;
  }

  .ink-search-input::placeholder {
    color: var(--ink-sub);
  }

  .ink-search-kbd {
    font-size: 10px;
    color: var(--ink-sub);
    flex: none;
    user-select: none;
  }

  .ink-search-clear {
    width: 18px;
    height: 18px;
    border: 0;
    padding: 0;
    border-radius: 50%;
    background: var(--ink-raised);
    color: var(--ink-sub);
    display: grid;
    place-items: center;
    cursor: pointer;
    flex: none;
  }

  .ink-search-clear:hover {
    color: var(--ink-text);
  }

  .ink-search-popover {
    position: absolute;
    left: 0;
    top: calc(100% + 6px);
    width: 286px;
    max-height: 310px;
    overflow-y: auto;
    z-index: 40;
    padding: 6px;
    border-radius: var(--ink-r-popover);
    background: var(--ink-surface);
    box-shadow: var(--ink-shadow-popover);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .ink-search-empty {
    margin: 0;
    padding: 12px 10px;
    font-size: 12px;
    color: var(--ink-sub);
    text-align: center;
  }

  .ink-search-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    padding: 7px 9px;
    border: 0;
    border-radius: var(--ink-r-row);
    background: transparent;
    color: var(--ink-text);
    font-family: inherit;
    text-align: left;
    cursor: pointer;
  }

  .ink-search-item[data-active='true'],
  .ink-search-item:hover {
    background: var(--ink-hover);
  }

  .ink-search-item-main {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .ink-search-item-title {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--ink-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ink-search-item-tab {
    font-size: 11px;
    color: var(--ink-sub);
  }

  .ink-nav {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 31px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--ink-r-row);
    background: transparent;
    color: var(--ink-text-2);
    font-family: inherit;
    font-size: 13px;
    text-align: left;
    white-space: nowrap;
    cursor: pointer;
    width: 100%;
    box-sizing: border-box;
  }

  .ink-nav:hover:not([data-active='true']) {
    background: var(--ink-hover);
    color: var(--ink-text);
  }

  .ink-nav[data-active='true'] {
    background: var(--ink-nav-on-bg);
    color: var(--ink-nav-on-text);
    font-weight: 500;
  }

  .ink-nav-spacer {
    flex: 1;
  }

  .ink-main {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-sizing: border-box;
  }

  @keyframes ink-search-pulse {
    0% {
      background-color: color-mix(in srgb, var(--ink-accent) 26%, transparent);
      box-shadow: inset 0 0 0 1.5px var(--ink-accent);
    }
    70% {
      background-color: color-mix(in srgb, var(--ink-accent) 14%, transparent);
      box-shadow: inset 0 0 0 1.5px var(--ink-accent);
    }
    100% {
      background-color: transparent;
      box-shadow: none;
    }
  }

  :global(.ink-search-flash) {
    border-radius: var(--ink-r-row);
    animation: ink-search-pulse 1.35s ease-out;
  }

  .ink-top-banners {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px 48px 0 26px;
  }

  .ink-close-btn {
    position: absolute;
    top: 12px;
    right: 12px;
    z-index: 3;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: var(--ink-close-r);
    background: var(--ink-close-bg);
    color: var(--ink-text-2);
    display: grid;
    place-items: center;
    cursor: pointer;
  }

  .ink-close-btn:hover {
    background: var(--ink-hover);
    color: var(--ink-text);
  }

  .ink-info-page {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .ink-info-head {
    flex: none;
    padding: 18px 60px 12px 26px;
  }

  .ink-info-title {
    margin: 0;
    font-family: var(--ink-font-display);
    font-size: 19px;
    font-weight: 600;
    color: var(--ink-text);
    user-select: none;
  }

  .ink-info-credit {
    margin-top: auto;
    padding-top: 12px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .ink-info-credit-app {
    margin: 0;
    font-size: 11px;
    line-height: 16px;
    color: var(--ink-sub);
    user-select: none;
  }

  .ink-info-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
    padding: 2px 20px 22px 26px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
</style>
