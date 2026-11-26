// 사용 설명서 카드 8장 메타(R2-HELP-1~4)

/**
 * @typedef {{
 *   id: 'capture' | 'markup' | 'draw' | 'board' | 'zoom' | 'record' | 'shortcuts' | 'troubleshoot',
 *   icon: string,
 *   shortcutId?: string,
 *   tryAction?: string,
 *   targetTab?: 'shortcuts' | 'general',
 *   stepKeys: string[],
 *   tipKeys: string[],
 *   diagram: Array<{ icon: string, labelKey: string, keyLabel?: string }>,
 * }} HelpGuideDef
 */

/** @type {HelpGuideDef[]} */
export const HELP_GUIDES = [
  {
    id: 'capture',
    icon: 'region',
    shortcutId: 'region',
    tryAction: 'capture.region',
    stepKeys: ['help.capture.s1', 'help.capture.s2', 'help.capture.s3', 'help.capture.s4'],
    tipKeys: ['help.capture.t1', 'help.capture.t2', 'help.capture.t3'],
    diagram: [
      { icon: 'shortcuts', labelKey: 'help.capture.d1' },
      { icon: 'region', labelKey: 'help.capture.d2' },
      { icon: 'check-circle', labelKey: 'help.capture.d3' },
    ],
  },
  {
    id: 'markup',
    icon: 'rect',
    shortcutId: 'pin',
    tryAction: 'capture.region',
    stepKeys: ['help.markup.s1', 'help.markup.s2', 'help.markup.s3', 'help.markup.s4'],
    tipKeys: ['help.markup.t1', 'help.markup.t2', 'help.markup.t3'],
    diagram: [
      { icon: 'rect', labelKey: 'help.markup.d1' },
      { icon: 'pin', labelKey: 'help.markup.d2' },
      { icon: 'copy', labelKey: 'help.markup.d3' },
    ],
  },
  {
    id: 'draw',
    icon: 'draw',
    shortcutId: 'draw',
    tryAction: 'draw',
    stepKeys: ['help.draw.s1', 'help.draw.s2', 'help.draw.s3', 'help.draw.s4'],
    tipKeys: ['help.draw.t1', 'help.draw.t2', 'help.draw.t3'],
    diagram: [
      { icon: 'draw', labelKey: 'help.draw.d1' },
      { icon: 'pen', labelKey: 'help.draw.d2' },
      { icon: 'restore', labelKey: 'help.draw.d3' },
    ],
  },
  {
    id: 'board',
    icon: 'board',
    shortcutId: 'board',
    tryAction: 'board',
    stepKeys: ['help.board.s1', 'help.board.s2', 'help.board.s3', 'help.board.s4'],
    tipKeys: ['help.board.t1', 'help.board.t2'],
    diagram: [
      { icon: 'board', labelKey: 'help.board.d1' },
      { icon: 'dark', labelKey: 'help.board.d2' },
      { icon: 'save', labelKey: 'help.board.d3' },
    ],
  },
  {
    id: 'zoom',
    icon: 'zoom',
    shortcutId: 'zoom',
    tryAction: 'zoom',
    stepKeys: ['help.zoom.s1', 'help.zoom.s2', 'help.zoom.s3', 'help.zoom.s4'],
    tipKeys: ['help.zoom.t1', 'help.zoom.t2', 'help.zoom.t3'],
    diagram: [
      { icon: 'zoom', labelKey: 'help.zoom.d1' },
      { icon: 'slider', labelKey: 'help.zoom.d2' },
      { icon: 'draw', labelKey: 'help.zoom.d3' },
    ],
  },
  {
    id: 'record',
    icon: 'record',
    shortcutId: 'record',
    tryAction: 'record.toggle',
    stepKeys: ['help.record.s1', 'help.record.s2', 'help.record.s3', 'help.record.s4'],
    tipKeys: ['help.record.t1', 'help.record.t2'],
    diagram: [
      { icon: 'region', labelKey: 'help.record.d1' },
      { icon: 'record', labelKey: 'help.record.d2' },
      { icon: 'folder', labelKey: 'help.record.d3' },
    ],
  },
  {
    id: 'shortcuts',
    icon: 'shortcuts',
    shortcutId: 'd-help',
    targetTab: 'shortcuts',
    stepKeys: ['help.shortcuts.s1', 'help.shortcuts.s2', 'help.shortcuts.s3', 'help.shortcuts.s4'],
    tipKeys: ['help.shortcuts.t1', 'help.shortcuts.t2'],
    diagram: [
      { icon: 'help-circle', labelKey: 'help.shortcuts.d1' },
      { icon: 'shortcuts', labelKey: 'help.shortcuts.d2' },
      { icon: 'check-circle', labelKey: 'help.shortcuts.d3' },
    ],
  },
  {
    id: 'troubleshoot',
    icon: 'warning-triangle',
    targetTab: 'general',
    stepKeys: ['help.troubleshoot.s1', 'help.troubleshoot.s2', 'help.troubleshoot.s3'],
    tipKeys: ['help.troubleshoot.t1', 'help.troubleshoot.t2'],
    diagram: [
      { icon: 'warning-triangle', labelKey: 'help.troubleshoot.d1' },
      { icon: 'settings', labelKey: 'help.troubleshoot.d2' },
      { icon: 'check-circle', labelKey: 'help.troubleshoot.d3' },
    ],
  },
];

export const TROUBLESHOOT_ITEMS = [
  {
    id: 'primary',
    titleKey: 'help.troubleshoot.item1Title',
    descKey: 'help.troubleshoot.item1Desc',
    target: 'checkup',
  },
  {
    id: 'hotkeys',
    titleKey: 'help.troubleshoot.item2Title',
    descKey: 'help.troubleshoot.item2Desc',
    target: 'checkup',
  },
  {
    id: 'zoom',
    titleKey: 'help.troubleshoot.item3Title',
    descKey: 'help.troubleshoot.item3Desc',
    target: 'checkup',
  },
  {
    id: 'reset',
    titleKey: 'help.troubleshoot.item4Title',
    descKey: 'help.troubleshoot.item4Desc',
    target: 'reset',
  },
];
