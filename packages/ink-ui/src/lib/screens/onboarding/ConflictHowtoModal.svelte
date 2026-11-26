<script>
  import { Dialog } from 'bits-ui';
  import Icon from '../../icons/Icon.svelte';
  import { getConflictHowtoKind } from '../../models/onboarding.js';
  import { t } from '../../i18n/index.svelte.js';

  /**
   * 겹치는 앱 단축키 끄는 법 안내 모달(R2-FIX-2).
   * FirstRunSheet(z-index 71) 위에서도 뜨도록 z-index 80/81을 쓴다.
   * @type {{
   *   open?: boolean,
   *   appName?: string,
   *   onOpenShortcuts?: () => void,
   * }}
   */
  let {
    open = $bindable(false),
    appName = '',
    onOpenShortcuts = undefined,
  } = $props();

  let kind = $derived(getConflictHowtoKind(appName));
  let titleText = $derived(
    appName
      ? t('onboarding.howto.title', { app: appName })
      : t('onboarding.howto.genericTitle'),
  );
  let steps = $derived.by(() => {
    if (kind === 'snipaste') {
      return [
        t('onboarding.howto.snipaste1'),
        t('onboarding.howto.snipaste2'),
        t('onboarding.howto.snipaste3'),
      ];
    }
    if (kind === 'powertoys') {
      return [
        t('onboarding.howto.powertoys1'),
        t('onboarding.howto.powertoys2'),
        t('onboarding.howto.powertoys3'),
      ];
    }
    return [
      t('onboarding.howto.generic1', { app: appName || 'App' }),
      t('onboarding.howto.generic2'),
      t('onboarding.howto.generic3'),
    ];
  });

  function handleOpenShortcuts() {
    open = false;
    onOpenShortcuts?.();
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Portal>
    <Dialog.Overlay class="ink-howto-overlay" />
    <Dialog.Content class="ink-howto-dialog" aria-label={titleText}>
      <div class="ink-howto-head">
        <span class="ink-howto-badge" aria-hidden="true">
          <Icon name="warning-triangle" size={18} />
        </span>
        <div class="ink-howto-titles">
          <Dialog.Title class="ink-howto-title">{titleText}</Dialog.Title>
          <Dialog.Description class="ink-howto-sub">{t('onboarding.howto.sub')}</Dialog.Description>
        </div>
      </div>

      <ol class="ink-howto-steps">
        {#each steps as stepText, idx (idx)}
          <li class="ink-howto-step">
            <span class="ink-howto-num ink-mono">{idx + 1}</span>
            <span class="ink-howto-text">{stepText}</span>
          </li>
        {/each}
      </ol>

      <div class="ink-howto-footer">
        {#if onOpenShortcuts}
          <button type="button" class="ink-howto-btn ink-howto-sec ink-focus" onclick={handleOpenShortcuts}>
            {t('onboarding.howto.openShortcuts')}
          </button>
        {/if}
        <Dialog.Close class="ink-howto-btn ink-howto-pri ink-focus">
          {t('onboarding.howto.close')}
        </Dialog.Close>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.ink-howto-overlay) {
    position: fixed;
    inset: 0;
    z-index: 80;
    background: var(--ink-sheet-scrim);
  }

  :global(.ink-howto-dialog) {
    position: fixed;
    left: 50%;
    top: 50%;
    z-index: 81;
    transform: translate(-50%, -50%);
    width: min(440px, calc(100vw - 32px));
    box-sizing: border-box;
    padding: 22px 24px 18px;
    border-radius: var(--ink-r-modal);
    background: var(--ink-surface);
    color: var(--ink-text);
    box-shadow: var(--ink-shadow-modal);
    font-family: var(--ink-font-sans);
    display: flex;
    flex-direction: column;
    gap: 16px;
    outline: none;
  }

  .ink-howto-head {
    display: flex;
    align-items: flex-start;
    gap: 12px;
  }

  .ink-howto-badge {
    width: 34px;
    height: 34px;
    border-radius: var(--ink-r-group);
    background: var(--ink-attn-bg);
    color: var(--ink-attn);
    display: grid;
    place-items: center;
    flex: none;
  }

  .ink-howto-titles {
    min-width: 0;
  }

  :global(.ink-howto-title) {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    line-height: 1.3;
    color: var(--ink-text);
  }

  :global(.ink-howto-sub) {
    margin: 4px 0 0;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--ink-sub);
  }

  .ink-howto-steps {
    margin: 0;
    padding: 12px 14px;
    list-style: none;
    border-radius: var(--ink-r-group);
    background: var(--ink-group-bg);
    box-shadow: var(--ink-group-ring);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .ink-howto-step {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    font-size: 13px;
    line-height: 1.45;
    color: var(--ink-text);
  }

  .ink-howto-num {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--ink-raised);
    color: var(--ink-text-2);
    font-size: 11px;
    font-weight: 600;
    display: grid;
    place-items: center;
    flex: none;
    margin-top: 1px;
  }

  .ink-howto-text {
    flex: 1;
    min-width: 0;
  }

  .ink-howto-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  :global(.ink-howto-btn) {
    height: 30px;
    padding: 0 14px;
    border: 0;
    border-radius: var(--ink-r-btn);
    font-family: inherit;
    font-size: 12.5px;
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
  }

  :global(.ink-howto-sec) {
    background: var(--ink-btn-sec-bg);
    color: var(--ink-text);
    box-shadow: var(--ink-btn-sec-shadow);
  }

  :global(.ink-howto-sec:hover) {
    background: var(--ink-hover);
  }

  :global(.ink-howto-pri) {
    background: var(--ink-invert);
    color: var(--ink-on-invert);
  }

  :global(.ink-howto-btn:active) {
    opacity: 0.85;
  }
</style>
