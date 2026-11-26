<script>
  import { Dialog } from 'bits-ui';
  import Icon from '../../icons/Icon.svelte';
  import Keycap from '../Keycap.svelte';
  import HelpDiagram from './HelpDiagram.svelte';
  import { TROUBLESHOOT_ITEMS } from './guides.js';
  import { t } from '../../i18n/index.svelte.js';

  /**
   * 사용 설명서 상세 모달(R2-HELP-2·3·4).
   * @type {{
   *   open?: boolean,
   *   guide: import('./guides.js').HelpGuideDef | null,
   *   combo?: string,
   *   platform?: 'windows' | 'macos',
   *   onTry?: (guide: import('./guides.js').HelpGuideDef) => void,
   *   onCheckup?: (target: 'checkup' | 'reset') => void,
   * }}
   */
  let {
    open = $bindable(false),
    guide = null,
    combo = '',
    platform = 'windows',
    onTry = undefined,
    onCheckup = undefined,
  } = $props();

  function handleTry() {
    if (!guide) return;
    open = false;
    onTry?.(guide);
  }

  function handleTroubleshootCheck(target) {
    open = false;
    onCheckup?.(target);
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Portal>
    <Dialog.Overlay class="ink-hmodal-overlay" />
    <Dialog.Content class="ink-hmodal-dialog" aria-label={guide ? t(`help.${guide.id}.title`) : ''}>
      {#if guide}
        <div class="ink-hmodal-head">
          <span class="ink-hmodal-icon">
            <Icon name={guide.icon} size={22} />
          </span>
          <div class="ink-hmodal-titles">
            <div class="ink-hmodal-title-row">
              <Dialog.Title class="ink-hmodal-title">{t(`help.${guide.id}.title`)}</Dialog.Title>
              {#if combo}
                <Keycap {combo} {platform} />
              {/if}
            </div>
            <Dialog.Description class="ink-hmodal-sub">
              {t(`help.${guide.id}.summary`)}
            </Dialog.Description>
          </div>
          <Dialog.Close class="ink-hmodal-x ink-focus" aria-label={t('help.modal.close')}>
            <Icon name="close" size={16} />
          </Dialog.Close>
        </div>

        <div class="ink-hmodal-body ink-scroll">
          <HelpDiagram nodes={guide.diagram} />

          <section class="ink-hmodal-sec">
            <h3 class="ink-hmodal-sec-title">{t('help.modal.steps')}</h3>
            <ol class="ink-hmodal-steps">
              {#each guide.stepKeys as key, idx (key)}
                <li class="ink-hmodal-step">
                  <span class="ink-hmodal-num ink-mono">{idx + 1}</span>
                  <span>{t(key)}</span>
                </li>
              {/each}
            </ol>
          </section>

          {#if guide.id === 'troubleshoot'}
            <section class="ink-hmodal-sec">
              <div class="ink-hmodal-trouble-list">
                {#each TROUBLESHOOT_ITEMS as item (item.id)}
                  <div class="ink-hmodal-trouble-row">
                    <div class="ink-hmodal-trouble-copy">
                      <p class="ink-hmodal-trouble-title">{t(item.titleKey)}</p>
                      <p class="ink-hmodal-trouble-desc">{t(item.descKey)}</p>
                    </div>
                    <button
                      type="button"
                      class="ink-hmodal-check-btn ink-focus"
                      onclick={() => handleTroubleshootCheck(item.target)}
                    >
                      {t('help.modal.check')}
                    </button>
                  </div>
                {/each}
              </div>
            </section>
          {/if}

          <section class="ink-hmodal-sec">
            <h3 class="ink-hmodal-sec-title">{t('help.modal.tips')}</h3>
            <ul class="ink-hmodal-tips">
              {#each guide.tipKeys as key (key)}
                <li>{t(key)}</li>
              {/each}
            </ul>
          </section>
        </div>

        <div class="ink-hmodal-footer">
          <Dialog.Close class="ink-hmodal-btn ink-hmodal-sec-btn ink-focus">
            {t('help.modal.close')}
          </Dialog.Close>
          <button type="button" class="ink-hmodal-btn ink-hmodal-pri-btn ink-focus" onclick={handleTry}>
            {guide.id === 'troubleshoot' ? t('help.troubleshoot.tryLabel') : t('help.modal.try')}
          </button>
        </div>
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.ink-hmodal-overlay) {
    position: fixed;
    inset: 0;
    z-index: 75;
    background: var(--ink-sheet-scrim);
  }

  :global(.ink-hmodal-dialog) {
    position: fixed;
    left: 50%;
    top: 50%;
    z-index: 76;
    transform: translate(-50%, -50%);
    width: min(520px, calc(100vw - 32px));
    max-height: min(620px, calc(100vh - 32px));
    box-sizing: border-box;
    padding: 22px 24px 18px;
    border-radius: var(--ink-r-modal);
    background: var(--ink-surface);
    color: var(--ink-text);
    box-shadow: var(--ink-shadow-modal);
    font-family: var(--ink-font-sans);
    display: flex;
    flex-direction: column;
    gap: 14px;
    outline: none;
  }

  .ink-hmodal-head {
    display: flex;
    align-items: flex-start;
    gap: 12px;
  }

  .ink-hmodal-icon {
    width: 38px;
    height: 38px;
    border-radius: var(--ink-r-group);
    background: var(--ink-raised);
    color: var(--ink-text);
    display: grid;
    place-items: center;
    flex: none;
  }

  .ink-hmodal-titles {
    flex: 1;
    min-width: 0;
  }

  .ink-hmodal-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  :global(.ink-hmodal-title) {
    margin: 0;
    font-family: var(--ink-font-display);
    font-size: 18px;
    font-weight: 600;
    color: var(--ink-text);
  }

  :global(.ink-hmodal-sub) {
    margin: 3px 0 0;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--ink-sub);
  }

  :global(.ink-hmodal-x) {
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: var(--ink-close-r);
    background: var(--ink-close-bg);
    color: var(--ink-text-2);
    display: grid;
    place-items: center;
    cursor: pointer;
    flex: none;
  }

  :global(.ink-hmodal-x:hover) {
    background: var(--ink-hover);
    color: var(--ink-text);
  }

  .ink-hmodal-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding-right: 2px;
  }

  .ink-hmodal-sec {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .ink-hmodal-sec-title {
    margin: 0;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--ink-sub);
  }

  .ink-hmodal-steps {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .ink-hmodal-step {
    display: flex;
    align-items: flex-start;
    gap: 9px;
    font-size: 13px;
    line-height: 1.45;
    color: var(--ink-text);
  }

  .ink-hmodal-num {
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

  .ink-hmodal-trouble-list {
    display: flex;
    flex-direction: column;
    border-radius: var(--ink-r-group);
    background: var(--ink-group-bg);
    box-shadow: var(--ink-group-ring);
    overflow: hidden;
  }

  .ink-hmodal-trouble-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-top: 1px solid var(--ink-hair);
  }

  .ink-hmodal-trouble-row:first-child {
    border-top: 0;
  }

  .ink-hmodal-trouble-copy {
    flex: 1;
    min-width: 0;
  }

  .ink-hmodal-trouble-title {
    margin: 0;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--ink-text);
  }

  .ink-hmodal-trouble-desc {
    margin: 2px 0 0;
    font-size: 11.5px;
    line-height: 1.4;
    color: var(--ink-sub);
  }

  .ink-hmodal-check-btn {
    height: 26px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--ink-r-btn);
    background: var(--ink-btn-sec-bg);
    box-shadow: var(--ink-btn-sec-shadow);
    color: var(--ink-text);
    font-family: inherit;
    font-size: 11.5px;
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
    flex: none;
  }

  .ink-hmodal-check-btn:hover {
    background: var(--ink-hover);
  }

  .ink-hmodal-tips {
    margin: 0;
    padding: 10px 14px 10px 28px;
    border-radius: var(--ink-r-group);
    background: var(--ink-raised);
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--ink-text-2);
  }

  .ink-hmodal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 2px;
  }

  :global(.ink-hmodal-btn) {
    height: 32px;
    padding: 0 16px;
    border: 0;
    border-radius: var(--ink-r-btn);
    font-family: inherit;
    font-size: 12.5px;
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
  }

  :global(.ink-hmodal-sec-btn) {
    background: var(--ink-btn-sec-bg);
    color: var(--ink-text);
    box-shadow: var(--ink-btn-sec-shadow);
  }

  :global(.ink-hmodal-sec-btn:hover) {
    background: var(--ink-hover);
  }

  .ink-hmodal-pri-btn {
    background: var(--ink-invert);
    color: var(--ink-on-invert);
    font-weight: 600;
  }

  :global(.ink-hmodal-btn:active) {
    opacity: 0.85;
  }
</style>
