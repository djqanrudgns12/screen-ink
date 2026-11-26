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
 * @property {number} pinnedCount   창 항상 위 고정(R-TOP) 개수
 * @property {number} [pins]        캡처 핀(R-CAP-29) 개수
 * @property {{active: boolean, elapsedMs: number}} recording
 * @property {Record<string, any>} settings   설정 값(05 §1 표의 키)
 * @property {ShortcutRow[]} shortcuts
 * @property {ConflictSummary} conflicts
 * @property {string} version
 * @property {boolean} readonly      더 새 버전 설정·쓸 수 없는 폴더 → 바꿔도 저장 안 함(안내 띠)
 * @property {{path: string, portable: boolean, readOnly: boolean}} location  설정 파일 위치(S05 데이터 > 설정 위치)
 * @property {'unpackaged'|'msix-store'|'msix-direct'} [channel]  설치 채널(MSIX 패키지/설치/포터블 구분)
 * @property {string} [autostartNotice]  MSIX StartupTask 차단(DisabledByUser·정책) 시 설정 화면 한 줄 안내
 * @property {string[]} notices      시작할 때 한 번 알려 줄 문구(손상 복구 등)
 * @property {Mode} [mode]          지금 모드(오버레이 창이 캡처·판서 화면을 고른다)
 *
 * @typedef {{mode: 'idle'}
 *   | {mode: 'capture', session: number, kind: 'region'|'window'|'record', startedMs: number}
 *   | {mode: 'draw', session: number, board: boolean, monitorId: number, startedMs: number}} Mode
 *   draw = Alt+1 판서(board=false) · Alt+3 칠판(board=true). 그 모니터(monitorId) 오버레이 창만 DrawOverlay를 띄운다.
 *
 * @typedef {Object} SetShortcutResult
 * @property {boolean} ok
 * @property {ShortcutStatus} status
 * @property {string} [reason]
 * @property {string[]} [suggestions]  줄마다 다른 추천 3개
 *
 * @typedef {'capture.region'|'capture.window'|'record.toggle'|'draw'|'zoom'|'board'|'pin.toggle'|'pin.releaseAll'|'draw.restore'|'folder.open'|'settings.open'|'firstRun.open'|'app.quit'|'snipaste.howto'
 *   |'folder.pick'|'logs.open'|'settings.export'|'capture.history.clear'|'capture.saveLast'|'capture.copyHex'|'pins.closeAll'
 *   |'settings.close'|'popover.close'|'island.hidden'|'capture.cancel'|'draw.acceptSuggestion'|'draw.undo'} InkAction
 *   draw.acceptSuggestion·draw.undo = 판서 섬 단추([바꾸기]·[되돌리기]) → Rust가 'drawCommand' 이벤트로 오버레이 엔진에
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
 * @property {string} [error]   "이 파일은 슉쇽 설정이 아니에요"
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
 * @property {string} [thumbUrl]    결과 섬 썸네일(ink://thumb/…, 작은 BMP)
 * @property {string} [buttonKey]   단추 옆 키캡(예: 'Ctrl+S')
 * @property {'R'|'Y'|'G'|'O'|'B'|'L'|'W'|'K'} [penColor]  판서 섬(variant 'draw')의 지금 색 점
 * @property {string} [penWidth]    판서 섬의 굵기('4px')
 * @property {{label: string, action: InkAction}} [chip]  추천 칩 하나(예: '#RRGGBB 복사' → 'capture.copyHex')
 *
 * @typedef {{x: number, y: number, w: number, h: number}} PhysRect  물리 px, 가상 화면 좌표
 *
 * @typedef {Object} AfterimageEvent  확정 직후 고른 자리 주황 잔상 0.4초(R-CAP-26)
 * @property {number} session
 * @property {PhysRect} rect
 * @property {number} durationMs
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
 * @property {PhysRect} [highlight] 첫 강조(물리 px) — 진입 순간 = 커서 모니터 전체(F1·F3), F2 = 커서 아래 창
 *
 * @typedef {Object} CaptureView  고르기 상태(Rust가 정함 → 모든 오버레이·섬이 같은 것을 그림)
 * @property {number} session
 * @property {'hover'|'drag'|'confirmed'} phase   hover = 인식 강조 · drag = 직접 그리기 · confirmed = 클릭으로 확정(손잡이·방향키)
 * @property {PhysRect|null} rect                  지금 범위(물리 px). 네이티브 정지 화면은 이미 이 자리를 밝게 뚫어 둠
 * @property {string[]} path                        사슬 이름(큰 것 → 작은 것, 예: ['창','표','2행']) — 창 제목·글자는 담지 않음
 * @property {number} level                         path 중 지금 단계(흰색으로)
 * @property {{x1:number,y1:number,x2:number,y2:number}[]} guides  자석이 붙은 변의 안내선(물리 px)
 * @property {boolean} magnet                       false = Alt로 자석 끔(섬에 "자석 끔")
 * @property {boolean} cursorIncluded               ` 키 — 크기 배지에 "커서 포함"
 * @property {boolean} help                         ? 키 — 섬 키 안내 둘째 줄을 강제로 보임
 * @property {{rect: PhysRect, label: string, score: number, sources: string, current: boolean, hidden: boolean}[]} [evidence]  숨은 설정 capture.evidence — 사슬 마디(작은 것 → 큰 것)의 근거·점수
 *
 * @typedef {{type:'move', x:number, y:number}
 *   | {type:'wheel', dy:-1|1}
 *   | {type:'down'|'up'|'dblclick', x:number, y:number, button:0|2}
 *   | {type:'key', key:string, code:string, ctrl:boolean, shift:boolean, alt:boolean, meta?:boolean}
 *   | {type:'command', name:'copy'|'save'|'quickSave'|'cancel'}} CaptureInput
 *   command = 선택 툴바 단추(R-CAP-27) — 키와 같은 결과(Rust가 판단). meta = 맥 Cmd.
 *   좌표는 물리 px(가상 화면) = monitor.bounds + client × scale. wheel dy = -1 위(넓히기·부모) / 1 아래(좁히기·자식).
 *   key는 e.key와 e.code를 같이(한글 입력 중에도 code로 판단). Esc·우클릭·Enter 등 판단은 모두 Rust가 한다.
 *
 * @typedef {Object} LiveZoomState  실시간 확대 HUD(Alt+2, 04 S13 · R-ZOOM-6) — 'zoom' 이벤트, 끝나면 null.
 *   InkState.mode = {mode:'liveZoom', session, monitorId, startedMs}. 좌표는 모두 Rust가 정한다.
 * @property {number} session
 * @property {number} monitorId
 * @property {string} monitorLabel   위치 지도 이름('모니터 1', 배치 순서 — 창 제목 없음)
 * @property {number} scale          1.25 · 1.5 · 2 · 2.5 · 3 · 4 · 6 · 8
 * @property {string} scaleLabel     섬·배지 글자('×2.0', '×1.25')
 * @property {{x: number, y: number, w: number, h: number}} viewport  위치 지도 0..1(원본 사각형 / 모니터)
 * @property {{x: number, y: number}} [cursorCss]  확대 화면 위 커서(HUD 창 안 CSS px) — 44px 주황 후광 중심. 확대 화면 밖이면 없음
 * @property {boolean} showMinimap   설정 zoom.minimap
 *
 * @typedef {{type:'wheel', dy:-1|1} | {type:'key', slot:'z-exit'} | {type:'exit'}} ZoomInput
 *   실시간 확대 입력(미리보기·HUD). wheel dy = -1 위(더 확대) / 1 아래(덜 확대). 배율은 휠로만, 키는 끝내기(z-exit) 하나.
 *   실제 앱의 휠은 Rust 돋보기 훅이 받는다(확대 모니터 안 세로 휠, Shift+휠은 아래 앱으로).
 */

// ───────────── 판서·칠판(A §2.5) ─────────────
// 경계: 포인터·획 보정·렌더·실행 취소는 웹뷰(inkEngine.js + DrawCanvas)가, 진입 캡처·보관·복원 판정·
// 클립보드/파일·섬·시스템 사건 종료는 Rust가 맡는다. 판서 문서는 웹뷰에만 있으므로 Rust가 먼저 끝낸 경우
// (같은 키·다른 모드 키·잠금·화면 구성 변경) 화면은 그 세션으로 drawFinish를 한 번 더 보내 보관한다(늦은 끝).

/**
 * @typedef {{s: number, tx: number, ty: number}} DrawViewTransform  확대 s(1~8)·이동(CSS px, ≤ 0) — A §2.1
 * @typedef {'none'|'same'|'rescaled'|'differentMonitor'} KeptCompat  보관 판서를 이 모니터에 불러올 수 있는지(02 §3.6)
 *
 * @typedef {Object} DrawSession  판서 한 번(`draw` 이벤트) — 상태(mode: draw)보다 먼저 온다
 * @property {number} session
 * @property {boolean} board                 false = Alt+1 판서(정지 화면 위), true = Alt+3 칠판(frameUrl 없음)
 * @property {OverlayMonitor} monitor        대상 모니터(bounds·scale·frameUrl). frameUrl = 그 순간 BMP(물리 px) — <img crossorigin="anonymous">
 * @property {string} layoutFingerprint      진입 순간 모니터 구성 지문
 * @property {DrawViewTransform} initialView 시작 확대·이동(기본 {s:1,tx:0,ty:0})
 * @property {boolean} restoreOnOpen         true면 열자마자 drawRestore(Alt+Shift+1). 판서 중 다시 누르면 같은 세션이 true로 한 번 더 온다
 * @property {boolean} hasKept               보관 판서가 있음
 * @property {KeptCompat} keptCompatible
 * @property {{x: number, y: number}} cursor 단축키 순간 커서(물리 px)
 *
 * @typedef {Object} KeptInk  보관 판서(메모리 한 장, 디스크에 쓰지 않음) — 모니터·지문·크기·배율은 Rust가 세션에서 채움
 * @property {number} monitorId
 * @property {string} layoutFingerprint
 * @property {number} physW
 * @property {number} physH
 * @property {number} scale          보관할 때 배율. Rescaled면 문서 비율 = (bounds.w / scale) / (physW / kept.scale)
 * @property {boolean} [board]       칠판에서 보관함
 * @property {any} document          InkDocument JSON v1(A §2.2)
 *
 * @typedef {Object} DrawFinishPayload
 * @property {number} session
 * @property {boolean} keep          보관할지(설정 draw.keep이 꺼져 있으면 Rust가 무시)
 * @property {{document: any}} [kept]  항목이 1개 이상일 때만 보관됨(다른 필드는 Rust가 채움)
 *
 * @typedef {{status: KeptCompat, kept?: KeptInk}} DrawRestoreResult  kept는 same·rescaled일 때만. 섬 문구는 Rust가 띄움
 *
 * @typedef {Object} DrawExportMeta  확대 중이어도 늘 모니터 전체(물리 해상도 그대로) — w·h가 대상 모니터 물리 크기와 다르면 거절
 * @property {number} w              물리 px = session.monitor.bounds.w
 * @property {number} h              = session.monitor.bounds.h
 * @property {number} [session]      있으면 지금 세션일 때만 받음
 * @property {'rgba'|'bgra'} [format] 기본 rgba(캔버스 getImageData 그대로)
 *
 * @typedef {Object} DrawIslandPatch  판서 섬 한 줄(보낸 것만 바뀜)
 * @property {number} [session]
 * @property {'R'|'Y'|'G'|'O'|'B'|'L'|'W'|'K'} [color]
 * @property {'pen'|'marker'|'text'|'select'|'eraser'|'pan'} [tool]
 * @property {number} [widthPx]      화면 기준 굵기 1~30
 * @property {number} [zoom]         1~8
 * @property {number} [pageIndex]
 * @property {number} [pageCount]
 * @property {'frame'|'white'|'black'} [background]
 * @property {boolean} [idleDim]     4초 무동작 물러남(섬 표현은 아직 없음)
 * @property {string} [notice]       보정·안내 한 줄('수평으로 맞췄어요' 등)
 * @property {'undo'} [noticeAction]  안내에 [되돌리기] 단추(모두 지우기 뒤) → drawCommand 'undo'
 * @property {string | null} [suggestion]  멈춤 도형 제안 글 — 조각마다 보낸다(없으면 걷힘). 섬 [바꾸기] → drawCommand 'acceptSuggestion'
 *
 * @typedef {{n: number, p50: number, p95: number, max: number}} PenSample  소프트웨어 펜 지연(ms) — M5
 *
 * @typedef {{session: number, command: 'acceptSuggestion'|'undo'}} DrawCommand  섬 단추 → 그 세션 오버레이의 엔진
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
 * @property {(id: number) => Promise<{w: number, h: number, url: string, zoom: number} | null>} pinInfo  캡처 핀(R-CAP-29) 창이 처음 묻는 것
 * @property {(id: number, action: 'close'|'closeAll'|'copy'|'save'|'drag'|'menu') => Promise<void>} pinAction  핀 동작(drag = 끌어 옮기기 시작, menu = 오른쪽 클릭 메뉴)
 * @property {(id: number, dy: -1|1, fx: number, fy: number) => Promise<number | null>} pinZoom  휠 한 칸 확대(-1)·축소(1), 커서 자리 = 창 안 비율 → 새 배율(%)
 * @property {(input: CaptureInput) => Promise<void>} captureInput  오버레이 입력 전달(포인터 이동은 rAF로 묶어 프레임당 한 번). 결과는 'capture' 이벤트로
 * @property {(session: number, paintMs?: number) => Promise<void>} drawReady  판서 배경을 그린 뒤 — 진입 계측 `mode enter draw [N ms]`(M4)
 * @property {(payload: DrawFinishPayload) => Promise<void>} drawFinish  판서 끝(Esc·우클릭 1×) · Rust가 먼저 끝낸 세션의 늦은 끝
 * @property {(session: number) => Promise<DrawRestoreResult>} drawRestore  Ctrl+Shift+R · restoreOnOpen
 * @property {(action: 'copy'|'save', bytes: ArrayBuffer|ArrayBufferView, meta: DrawExportMeta) => Promise<{ok: boolean, name?: string, error?: string}>} drawExport
 *   합성 결과(배경 + 판서) 원본 바이트를 base64 없이(원본 바이트 IPC). bytes 길이 = w × h × 4
 * @property {(patch: DrawIslandPatch) => Promise<void>} drawIsland
 * @property {(sample: PenSample) => Promise<void>} drawPenMetrics  로그 한 줄 `pen latency n=.. p50=.. p95=.. max=.. ms`
 * @property {(input: ZoomInput) => Promise<void>} zoomInput  실시간 확대 입력(휠·확대 안 키·끝). 결과는 'zoom'·'island' 이벤트로
 * @property {(handler: (e: {type: 'state', state: InkState} | {type: 'island', island: IslandEvent} | {type: 'overlay', overlay: OverlaySession} | {type: 'capture', view: CaptureView} | {type: 'afterimage', afterimage: AfterimageEvent} | {type: 'zoom', zoom: LiveZoomState | null} | {type: 'draw', draw: DrawSession} | ({type: 'drawCommand'} & DrawCommand)) => void) => () => void} subscribe
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
