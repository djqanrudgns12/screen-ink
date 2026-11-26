// @screen-ink/ui 공개 출구 — 디자인 토큰·아이콘·부품 15종·화면 7종·순수 모델

// 아이콘
export { ICONS, ICON_NAMES } from './icons/icons.js';
export { default as Icon } from './icons/Icon.svelte';
export { default as AppIcon } from './icons/AppIcon.svelte';

// 부품
export { default as TrayPopover } from './components/TrayPopover.svelte';
export { default as MenuRow } from './components/MenuRow.svelte';
export { default as SectionLabel } from './components/SectionLabel.svelte';
export { default as Group } from './components/Group.svelte';
export { default as Row } from './components/Row.svelte';
export { default as Keycap } from './components/Keycap.svelte';
export { default as Switch } from './components/Switch.svelte';
export { default as Segmented } from './components/Segmented.svelte';
export { default as Banner } from './components/Banner.svelte';
export { default as Sheet } from './components/Sheet.svelte';
export { default as Island } from './components/Island.svelte';
export { default as SelectionFrame } from './components/SelectionFrame.svelte';
export { default as Magnifier } from './components/Magnifier.svelte';
export { default as MiniMap } from './components/MiniMap.svelte';
export { default as PinFrame } from './components/PinFrame.svelte';
export { default as MakerCredit } from './components/MakerCredit.svelte';

// 화면
export { default as TrayPopoverScreen } from './screens/TrayPopoverScreen.svelte';
export { default as SettingsWindow } from './screens/SettingsWindow.svelte';
export { default as ShortcutsPage } from './screens/ShortcutsPage.svelte';
export { default as CapturePage } from './screens/CapturePage.svelte';
export { default as DrawZoomPage } from './screens/DrawZoomPage.svelte';
export { default as GeneralPage } from './screens/GeneralPage.svelte';
export { default as FirstRunSheet } from './screens/FirstRunSheet.svelte';
export { default as CheatSheet } from './screens/CheatSheet.svelte';
export { default as IslandHost } from './screens/IslandHost.svelte';
export { default as CaptureOverlay } from './screens/CaptureOverlay.svelte';
export { default as DrawOverlay } from './screens/DrawOverlay.svelte';
export { default as PinView } from './screens/PinView.svelte';
export { default as ZoomHud } from './screens/ZoomHud.svelte';

// 순수 모델
export { eventToCombo, isEscapeOnly, isClearOnly } from './models/keyCombo.js';
export {
  ISLAND_DURATIONS,
  HINT_MAX_AUTO_SHOW,
  ISLAND_SPRING,
  createIslandState,
  getAutoDismissMs,
  pushIsland,
  dismissIsland,
  clearModeIsland,
  setIslandHover,
  setHelpOpen,
  shouldShowHintLine,
  formatCapturePath,
  buildCaptureIslandPatch,
  applyCaptureView,
} from './models/islandQueue.js';
export {
  MAGNIFIER_LENS,
  findMonitorForOverlay,
  clientToPhysical,
  physicalToClient,
  isPhysicalPointInMonitor,
  isPhysicalRectOverlappingMonitor,
  physicalRectToClient,
  resolveMonitorHighlight,
  resolveCaptureFrame,
  formatRectSize,
  clipGuideToMonitor,
  resolveMonitorGuides,
  toCapturePointerInput,
  toCaptureWheelInput,
  shouldPreventCaptureKey,
  toCaptureKeyInput,
  clientToImagePixel,
  computeSampleRect,
  buildMagnifierPatch,
  computeMagnifierPosition,
  clampByte,
  formatHex,
  formatRgb,
  formatColor,
  toggleColorFormat,
} from './models/captureOverlay.js';
export { resolveTheme, applyTheme, watchSystemTheme } from './models/theme.js';
export {
  validateFilenameTemplate,
  renderFilenamePreview,
  shortenPath,
} from './models/template.js';
export {
  SETTING_LABELS,
  formatSettingValue,
  describeSettingChange,
  formatLocationSub,
  formatFirstRunConflict,
} from './models/settingsMeta.js';
export {
  subjectParticle,
  formatComboReadable,
  formatConflictBanner,
  getShortcutRowDisplay,
} from './models/shortcutDisplay.js';
export { buildCheatSheetColumns } from './models/cheatSheet.js';

// 판서 엔진(순수 JS — A §2, 4.1)
export {
  INK_SCHEMA_VERSION,
  MAX_UNDO,
  INK_COLORS,
  INK_COLOR_CODES,
  INK_NOTICES,
  MARKER_OPACITY,
  InkEngine,
  createInkEngine,
  createInkDocument,
  documentOptionsFromSession,
  applyCommand,
  invertCommand,
  rescaleDocument,
  itemCount,
} from './ink/inkEngine.js';
export {
  ZOOM_STEPS,
  START_ZOOM_CHOICES,
  IDENTITY_VIEW,
  docToView,
  viewToDoc,
  docToCanvasPx,
  docRectToView,
  docSizeOf,
  physToDoc,
  docToPhys,
  docRectToPhys,
  clampView,
  zoomAt,
  nextZoomStep,
  panBy,
  centerOn,
  edgePushDelta,
  screenWidthToDoc,
  visibleDocFraction,
} from './ink/coords.js';
export { DRAW_KEY_IDS, LATER_DRAW_KEYS, buildDrawKeymap, matchDrawKey, normalizeCombo } from './ink/drawKeys.js';

// 판서 렌더러(4.2) — DrawCanvas + 2D 그리기 순수 함수 + M5 펜 지연 계측
export { default as DrawCanvas } from './components/DrawCanvas.svelte';
export { SUGGEST_TIMEOUT_MS, DEFAULT_MOSAIC_BLOCK } from './ink/inkEngine.js';
export {
  BOARD_COLORS,
  resolveInkColor,
  drawBackground,
  drawItem,
  renderItems,
  bakeItem,
  renderActive,
  renderDocument,
  createCanvasTextMeasurer,
} from './ink/canvasRender.js';
export { createPenLatencyMeter, summarizeLatency } from './ink/penMetrics.js';
export { HALO_SIZE, MINIMAP_MARGIN, isLiveZoomMode, applyZoomEvent, haloBox, showMinimap } from './models/zoomHud.js';
