// InkClient — UI(ink-ui)가 앱 핵심과 이야기하는 유일한 문(PRD R-ARCH-1, 03 §1).
// 단독 앱과 Tidy 플러그인, 브라우저 미리보기(Fake)가 같은 모양을 지킨다.

/**
 * @typedef {'windows'|'macos'} Platform
 * @typedef {'system'|'light'|'dark'} ThemePref
 * @typedef {'global'|'capture'|'draw'|'zoom'} ShortcutScope
 * @typedef {'ok'|'warn'|'conflict'|'off'} ShortcutStatus
 *
 * @typedef {Object} ShortcutRow
 * @property {string} id            확정본 칸 id(예: 'region', 'd-pen')
 * @property {ShortcutScope} scope
 * @property {string} label         화면에 쓰는 이름(12자 안)
 * @property {string} group         묶음 id: 'capture' | 'tools' | 'quick' | 'capture-mode' | 'draw-mode' | 'zoom-mode'
 * @property {string} combo         지금 값('F1', 'Alt+Shift+D', 빈 문자열 = 비움)
 * @property {ShortcutStatus} status
 * @property {string} [reason]      막힘·주의·겹침 이유 한 줄
 * @property {string} [owner]       겹친 앱 이름(Snipaste 등)
 * @property {boolean} [later]      후속 기능(딱지 "후속")
 * @property {boolean} [windowsOnly]
 *
 * @typedef {Object} ConflictSummary
 * @property {number} count
 * @property {string} [app]         원인이 대체 앱 하나면 그 이름 → "끄는 법" 먼저
 * @property {string[]} combos
 *
 * @typedef {Object} InkState
 * @property {Platform} platform
 * @property {ThemePref} theme
 * @property {boolean} hotkeysEnabled
 * @property {number} pinnedCount
 * @property {{active: boolean, elapsedMs: number}} recording
 * @property {Record<string, any>} settings   설정 값(05 §1 표의 키)
 * @property {ShortcutRow[]} shortcuts
 * @property {ConflictSummary} conflicts
 * @property {string} version
 * @property {boolean} readonly      더 새 버전 설정·쓸 수 없는 폴더 → 바꿔도 저장 안 함(안내 띠)
 * @property {{path: string, portable: boolean, readOnly: boolean}} location  설정 파일 위치(S05 데이터 > 설정 위치)
 * @property {string[]} notices      시작할 때 한 번 알려 줄 문구(손상 복구 등)
 *
 * @typedef {Object} SetShortcutResult
 * @property {boolean} ok
 * @property {ShortcutStatus} status
 * @property {string} [reason]
 * @property {string[]} [suggestions]  줄마다 다른 추천 3개
 *
 * @typedef {'capture.region'|'capture.window'|'record.toggle'|'draw'|'zoom'|'board'|'pin.toggle'|'pin.releaseAll'|'draw.restore'|'folder.open'|'settings.open'|'firstRun.open'|'app.quit'|'snipaste.howto'
 *   |'folder.pick'|'logs.open'|'settings.export'|'capture.history.clear'
 *   |'settings.close'|'popover.close'|'island.hidden'|'capture.cancel'} InkAction
 *   capture.cancel = 오버레이에서 Esc·우클릭(모드 끝, 정지 이미지 버림)
 *   folder.pick = 저장 폴더 [바꾸기](폴더 고르기 → output.folder) · logs.open = 로그 폴더 · settings.export = 내보내기(저장 대화)
 *   settings.close·popover.close·island.hidden = 창 닫기(앱 셸이 씀)
 *
 * @typedef {Object} SettingChange
 * @property {string} path     'draw.width' 또는 'shortcuts.<칸 id>'
 * @property {any} before
 * @property {any} after
 *
 * @typedef {Object} ImportPick  가져오기 1단계(파일 고르기 → 바뀌는 항목)
 * @property {boolean} cancelled
 * @property {string} [path]
 * @property {boolean} [ok]
 * @property {string} [error]   "이 파일은 screen-ink 설정이 아니에요"
 * @property {SettingChange[]} [changes]
 * @property {string[]} [warnings]
 *
 * @typedef {Object} IslandEvent  화면 위 섬(R-UI-1·5)
 * @property {'result'|'problem'|'info'|'mode'|'off'|'end'} kind   end = 모드가 끝남 → 모드 섬을 거둠
 * @property {string} text
 * @property {string} [detail]
 * @property {{label: string, action: InkAction|'undo'}} [button]
 * @property {string} [variant]    region·window·record·security … (없으면 kind로)
 * @property {string} [mode]       모드 섬의 모드 이름(region·window·record) — 키 안내 둘째 줄 횟수 세기
 * @property {string} [value]      고정폭 값(크기 등)
 * @property {{key: string, label: string}[]} [hints]  둘째 줄 키 안내
 *
 * @typedef {{x: number, y: number, w: number, h: number}} PhysRect  물리 px, 가상 화면 좌표
 *
 * @typedef {Object} OverlayMonitor
 * @property {number} id
 * @property {PhysRect} bounds
 * @property {PhysRect} work
 * @property {number} scale         1 = 100%, 1.5 = 150%
 * @property {boolean} primary
 * @property {string} [frameUrl]    정지 이미지(BMP, 물리 px 그대로). <img crossorigin="anonymous">로 — 캔버스 픽셀 읽기 가능
 *
 * @typedef {Object} OverlaySession  F1·F2·F3 진입 한 번(정지 스냅숏)
 * @property {number} session
 * @property {'region'|'window'|'record'} kind
 * @property {OverlayMonitor[]} monitors   오버레이 창마다 자기 monitor id 것만 그림
 * @property {{x: number, y: number}} cursor  단축키 순간 커서(물리 px)
 * @property {boolean} nativeFrame  true = 네이티브 창이 이미 정지 화면(50% 어둡게 + 강조만 밝게)을 그림 →
 *   UI는 이미지·어둡게를 깔지 않고 투명 층(테두리·꺾쇠·확대경·크기)만. 이미지는 확대경용으로 늦게 읽음
 * @property {PhysRect} [highlight] 첫 강조(물리 px) — 단계 2는 커서 아래 창
 */

/**
 * @typedef {Object} InkClient
 * @property {() => Promise<InkState>} getState
 * @property {(path: string, value: any) => Promise<{ok: boolean, error?: string, state: InkState}>} updateSetting
 * @property {(id: string, combo: string) => Promise<SetShortcutResult>} setShortcut
 * @property {(id: string, combo: string) => Promise<{status: ShortcutStatus, reason?: string}>} previewShortcut  누르는 동안 미리 판정(등록 안 함)
 * @property {(group: string) => Promise<InkState>} resetShortcutGroup
 * @property {() => Promise<{ok: boolean, state: InkState}>} undo   설정 창 안 Ctrl+Z
 * @property {(enabled: boolean) => Promise<InkState>} setHotkeysEnabled
 * @property {(action: InkAction) => Promise<void>} run
 * @property {() => Promise<ImportPick>} previewImport   가져오기: 파일 고르기 + 바뀌는 항목(저장 안 함)
 * @property {(path: string) => Promise<{ok: boolean, error?: string, state: InkState}>} applyImport  시트 [가져오기] — 되돌리기 1건
 * @property {() => Promise<InkState>} resetAll   모두 처음으로(확인 시트 뒤)
 * @property {(session: number, ui?: {loadMs?: number, paintMs?: number}) => Promise<void>} overlayReady  오버레이가 정지 이미지를 **그린 뒤**(load + 다음 프레임) — 진입 계측 M1. ui = 세션 알림→load, load→그림(선택)
 * @property {(handler: (e: {type: 'state', state: InkState} | {type: 'island', island: IslandEvent} | {type: 'overlay', overlay: OverlaySession}) => void) => () => void} subscribe
 */

export const SHORTCUT_GROUPS = [
  { id: 'capture', title: '캡처·녹화', scope: 'global' },
  { id: 'tools', title: '화면 도구', scope: 'global' },
  { id: 'quick', title: '빠른 동작', scope: 'global' },
  { id: 'capture-mode', title: '캡처 화면 안', scope: 'capture' },
  { id: 'draw-mode', title: '판서 화면 안', scope: 'draw' },
  { id: 'zoom-mode', title: '실시간 확대 안', scope: 'zoom' },
];

/** 키 조합을 화면용 키캡 배열로(Windows 'Shift', 맥 기호). */
export function keycaps(combo, platform = 'windows') {
  if (!combo) return [];
  const mac = { Ctrl: '⌃', Option: '⌥', Alt: '⌥', Shift: '⇧', Cmd: '⌘' };
  return combo.split('+').map((k) => (platform === 'macos' && mac[k] ? mac[k] : k));
}
