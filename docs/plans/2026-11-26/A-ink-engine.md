# A. 단계 4 판서·칠판 구현 시안 (`4.1`~`4.8`)

대상 항목(`progress.json`): `4.1` `inkEngine.js`(순수) · `4.2` `DrawCanvas` 렌더러 + M5(G5) · `4.3` 도구·도형·색·굵기 키·칠판 · `4.4` 휠 확대·가장자리 밀기·스마트 우클릭 · `4.5` 글자 T(IME 보호)·고르기·다시 고치기 · `4.6` 나가기·보관·복원 · `4.7` 판서 결과 복사·저장 · `4.8` 판서 섬·펜 색 고리·MiniMap·칠판 미리 보기.

---

## 1. 목표 · 범위 (요구사항 ID)

| 요구사항 | 우선 | 요약 |
|---|---|---|
| `R-DRAW-0` | P0 | `Alt+1` 커서 모니터 정지 → 판서, 휠 커서 중심 확대(1~8×), 가장자리 밀기, **스마트 우클릭**(확대 중 = 손 모드·좌클릭 복귀, 1× = 끝내기) |
| `R-DRAW-1` | P0 | 실행 취소·다시 실행(`Ctrl+Z` / `Ctrl+Y`, 맥 `Cmd+Z` / `Shift+Cmd+Z`), 최대 200단계, 명령 패턴 |
| `R-DRAW-2` | P0 | 나갈 때 마지막 판서 1장 메모리 보관(`keptInk`) → `Ctrl+Shift+R` 또는 `Alt+Shift+1`로 복원(모니터·배율 규칙) |
| `R-DRAW-3` | P0 | 화면 안 판서 키 29칸(`d-*`) 전부 설정값(`R-KEY-6`)에서 읽어 판정 + 도형 수정 키(`Shift` 직선 · `Ctrl` 사각 · `Ctrl+Shift` 화살표 · `Tab` 타원 · `+Alt` 점선) |
| `R-DRAW-4` | P0 | 글자(`T`): 한글 IME 조합 중(`compositionstart`~`compositionend`) 도구·색 단축키 무시, `Ctrl+T` 글꼴/크기 |
| `R-DRAW-5` | P0 | **M5 펜 지연 p95 ≤ 50 ms** (관문 G5, 학교 PC) |
| `R-DRAW-6` | P1 | 떨림 보정(끔/약/강 — One Euro 필터), 직선·화살표 ±4° 맞춤, 획 끝 0.5초 멈춤 도형 자동 판정 |
| `R-DRAW-9` | P0 | 시작 확대 배율(1.25·1.5·1.75·2·3·4), 휠 단계 `1 → 1.25 → 1.5 → 1.75 → 2 → 3 → 4 → 6 → 8` |
| `R-DRAW-10` | P0 | Esc 동작 선택: `step`(글자 입력 → 고르기 해제 → 확대 해제 → 나가기) / `exit`(바로 나가기) |
| `R-BOARD-1` | P0 | `Alt+3` 빈 칠판(기본 흰색), `W`/`K` 전환(검은 칠판에서 검정 펜은 흰색 자동 치환), 판서 도구 동일 |
| `R-BOARD-2` | P1 | 칠판 여러 장(`PgDn` 새 장 · `PgUp`/`PgDn` 넘기기) — 데이터 구조에 미리 자리 확보, 기본 1장 |
| `R-ZOOM-6` | P1 | 확대 중 오른쪽 아래 `MiniMap`(96×60, 주황 사각 = 보이는 영역) |
| `R-UI-1·5` | P0 | 판서 섬(한 줄, 지금 색 점 + 0.8초 6색 펼침, 4초 무동작 시 40% 물러남, 커서 접근 시 비켜 감) |

> 후속(1차 제외, 키 슬롯은 존재하되 미동작): `R-DRAW-7`(화면 강조 `H`, 체크·번호 도장 `F6`·`F7`), `R-DRAW-11`(3겹 커서 강조), `R-TOUCH-1`(전자칠판 팜 리젝션).

---

## 2. 정확한 형식

### 2.1 좌표 공간 규칙

판서 엔진은 **세 가지 좌표 공간**을 엄격히 분리한다. 변환 함수는 `packages/ink-ui/src/lib/ink/coords.js` 한 곳에만 둔다.

| 공간 | 단위 | 원점 | 쓰임 |
|---|---|---|---|
| **가상 화면 물리 px (`Phys`)** | 정수 물리 px | 주 모니터 왼쪽 위 `(0, 0)` | Rust `MonitorLayout`·`Frame`·`Snapshot`·결과 크롭 |
| **문서 공간 (`Doc`)** | 실수 논리 px (`0 .. monitor.bounds.w / scale`, `0 .. monitor.bounds.h / scale`) | **대상 모니터 왼쪽 위 `(0, 0)`** | **`InkDocument`의 모든 점·도형·글자·굵기 저장 단위** |
| **화면 CSS 공간 (`View`)** | 실수 CSS px (`0 .. viewportW`, `0 .. viewportH`) | 오버레이 웹뷰 왼쪽 위 `(0, 0)` | `PointerEvent.clientX/Y`, 커서 고리, 글자 입력 상자 배치 |

**왜 문서 공간을 "모니터 논리 px"로 두는가?**
- 같은 24인치 모니터에서 배율이 100% → 150%로 바뀌거나, `Alt+1` 판서 보관(`keptInk`)을 복원할 때 물리 해상도(`w`, `h`)와 `scale`을 함께 저장하면, 배율만 바뀐 경우 `docW_new / docW_old` 비율 한 번으로 획·글자 위치와 굵기가 정확히 스케일된다(`02 §3.6`).
- 캡처 주석(묶음 B, `4.9`)은 선택 범위 `rect`의 왼쪽 위 `(rect.x, rect.y)`를 문서 원점 `(0, 0)`으로, 문서 크기 `(rect.w / scale, rect.h / scale)`로 두면 **같은 `inkEngine.js`가 수정 0줄로 동작**한다.

**변환 공식 (`view = { s, tx, ty }`)**:
- `s`: 확대 배율 (`1.0 .. 8.0`). `tx, ty`: 화면 CSS 공간에서 문서 원점 `(0, 0)`이 놓이는 위치(CSS px, 항상 `<= 0`).
- `docToView(dx, dy, view)` = `{ vx: dx * view.s + view.tx, vy: dy * view.s + view.ty }`
- `viewToDoc(vx, vy, view)` = `{ dx: (vx - view.tx) / view.s, dy: (vy - view.ty) / view.s }`
- `docToCanvasPx(dx, dy, view, dpr)` = `{ cx: (dx * view.s + view.tx) * dpr, cy: (dy * view.s + view.ty) * dpr }`
- **커서 고정점 휠 확대 (`02 §3.5`)**: 커서 위치 `c = (vx, vy)`에서 배율 `s → s'`로 바꿀 때:
  ```js
  tx' = clamp(vx - (s' / s) * (vx - tx), viewportW - docW * s', 0)
  ty' = clamp(vy - (s' / s) * (vy - ty), viewportH - docH * s', 0)
  ```
- **화면 기준 굵기 (`02 §3.2`)**: 사용자가 고른 화면 굵기 `screenWidthPx`(1~30)와 현재 배율 `view.s`로 획 생성 순간 문서 굵기를 고정한다:
  ```js
  docWidth = Math.max(0.25, screenWidthPx / view.s)
  ```
  확대 `×2`에서 `4px`로 쓰면 문서에는 `2.0`으로 저장되고, 확대를 `×1`로 풀면 `2px`로 보여 확대 중 보던 상대 굵기 비율을 그대로 지킨다.

---

### 2.2 판서 문서 JSON 형식 (`InkDocument`, 스키마 v1)

순수 JS(`inkEngine.js`)와 Rust(`keptInk` 보관·복원 시 직렬화)가 같은 JSON 모양을 쓴다.

```json
{
  "schemaVersion": 1,
  "id": "draw-17",
  "mode": "draw",
  "monitorId": 0,
  "layoutFingerprint": "0:0,0,1920,1080@100|1:1920,0,2880,1800@150",
  "docSize": { "w": 1920, "h": 1080 },
  "physSize": { "w": 1920, "h": 1080 },
  "scale": 1.0,
  "frameId": 17,
  "background": "frame",
  "pages": [
    {
      "id": "page-1",
      "background": "frame",
      "items": [
        {
          "id": "it-1",
          "kind": "stroke",
          "tool": "pen",
          "color": "R",
          "width": 4.0,
          "opacity": 1.0,
          "dashed": false,
          "points": [
            { "x": 120.5, "y": 240.0, "p": 0.5, "t": 0 },
            { "x": 126.2, "y": 241.3, "p": 0.62, "t": 8 }
          ],
          "bbox": { "x": 118.5, "y": 238.0, "w": 9.7, "h": 5.3 }
        },
        {
          "id": "it-2",
          "kind": "shape",
          "shape": "arrow",
          "color": "O",
          "width": 4.0,
          "opacity": 1.0,
          "dashed": false,
          "x1": 300.0, "y1": 180.0,
          "x2": 540.0, "y2": 180.0,
          "headLen": 14.0,
          "bbox": { "x": 298.0, "y": 170.0, "w": 244.0, "h": 20.0 }
        },
        {
          "id": "it-3",
          "kind": "text",
          "color": "R",
          "fontFamily": "IBM Plex Sans KR",
          "fontSize": 24.0,
          "x": 420.0, "y": 310.0,
          "w": 180.0, "h": 34.0,
          "text": "핵심 공식",
          "bbox": { "x": 420.0, "y": 310.0, "w": 180.0, "h": 34.0 }
        }
      ]
    }
  ],
  "activePage": 0,
  "view": { "s": 1.0, "tx": 0.0, "ty": 0.0 }
}
```

**항목(`Item`) 종류 4종** (묶음 B의 모자이크까지 한 유니온으로 수용):

| `kind` | 필드 | 비고 |
|---|---|---|
| `"stroke"` | `id, tool: 'pen'\|'marker', color, width, opacity(pen=1, marker=0.38), dashed, points:[{x,y,p,t}], bbox` | 형광펜(`marker`)은 `globalCompositeOperation = 'multiply'`(흰/밝은 배경) 또는 `alpha 0.38` + `lineCap='butt'`(`04 S12`) — 검은 칠판에서는 `source-over` alpha 0.45로 가시성 확보 |
| `"shape"` | `id, shape: 'line'\|'rect'\|'ellipse'\|'arrow', color, width, opacity, dashed, x1, y1, x2, y2, headLen?, bbox` | `Shift`/`Ctrl`/`Tab`/`Ctrl+Shift` 드래그 또는 0.5초 멈춤 자동 보정 결과 |
| `"text"` | `id, color, fontFamily, fontSize(=screenWidth×6/s), x, y, w, h, text, bbox` | `T` 도구. 빈 문자열(`text.trim()===''`)은 커밋 금지 |
| `"mosaic"` | `id, x, y, w, h, blockSize, bbox` | **묶음 B(`4.9` 캡처 주석) 전용**. 판서 엔진에 타입만 예약해 두고, `DrawCanvas` 렌더러가 배경 이미지를 픽셀화해 그림 |

**지우개(`eraser`) 모델**:
- **획 단위 지우개(Object Eraser)**를 기본으로 채택한다(`05 §6`: "D → 획 위 끌기 → 닿은 항목 `Remove`").
- 포인터를 누르고 끄는 한 번의 지우개 제스처(`pointerdown`~`pointerup`) 동안 닿은 항목들을 모아 **`Remove` 명령 1개**로 묶어 Undo 스택에 넣는다(지우개로 3획을 한 번에 문질러 지우면 `Ctrl+Z` 1번에 3획이 함께 복원).
- 지우개 반경 = `Math.max(8, screenWidth * 1.5) / view.s` 문서 px.

**색 팔레트 상수 (`INK_COLORS`, `10-design-system.md §2`)**:
```js
export const INK_COLORS = Object.freeze({
  R: '#E5484D', Y: '#F2B705', G: '#2BA36B', O: '#F97316',
  B: '#2F6FED', L: '#84CC16', W: '#FFFFFF', K: '#1A1A1A',
});
```
검은 칠판(`background === 'black'`)에서 `color === 'K'`로 그리면 표시 색만 `#FFFFFF`로 치환한다(`04 S14`).

---

### 2.3 명령 · 실행 취소 / 다시 실행 모델 (`Command`, 최대 200단계)

모든 문서 변경은 순수 리듀서 `applyCommand(doc, cmd)` / `invertCommand(doc, cmd)`를 통해서만 일어난다(`05 §6`).

```js
/**
 * @typedef {
 *   | { type: 'add', page: number, item: Item }
 *   | { type: 'remove', page: number, entries: Array<{ index: number, item: Item }> }
 *   | { type: 'move', page: number, ids: string[], dx: number, dy: number }
 *   | { type: 'resize', page: number, id: string, before: Rect, after: Rect, fontBefore?: number, fontAfter?: number }
 *   | { type: 'setAttr', page: number, ids: string[], patch: Record<string, any>, prev: Record<string, Record<string, any>> }
 *   | { type: 'correct', page: number, before: Item, after: Item }
 *   | { type: 'clear', page: number, items: Item[] }
 *   | { type: 'setBg', page: number, before: 'frame'|'white'|'black', after: 'frame'|'white'|'black' }
 *   | { type: 'addPage', index: number, page: BoardPage }
 * } InkCommand
 */
```

- **Undo 스택 상한 = 200** (`MAX_UNDO = 200`, `05 §6`). 201번째 명령이 쌓이면 가장 오래된 명령 1개를 버린다(화면 항목은 그대로 남고 굳혀진 상태 유지).
- **보정(`correct`)과 `Ctrl+Z`**:
  1. 펜 획이 끝나면 먼저 `add(rawStroke)`를 쌓는다.
  2. 직선·화살표 ±4° 맞춤이나 0.5초 멈춤 도형 보정이 일어나면 **`correct(rawStroke, shapedItem)`을 별도 명령으로 쌓는다**(`02 §3.3`).
  3. 따라서 `Ctrl+Z` 1회 = 보정 전 원래 손획으로 복귀, `Ctrl+Z` 2회 = 획 자체 삭제.
- **새 명령 추가 시**: `redo` 배열을 비운다.

---

### 2.4 모듈 · 파일 위치

| 층 | 파일 경로 | 책임 |
|---|---|---|
| **순수 JS 엔진** | `packages/ink-ui/src/lib/ink/coords.js` | `Phys ↔ Doc ↔ View` 변환, 휠 고정점 확대·가두기, 줌 단계 상수 |
| **순수 JS 엔진** | `packages/ink-ui/src/lib/ink/oneEuro.js` | One Euro 필터 (`off` / `light` / `strong`), 짧은 획 예외 |
| **순수 JS 엔진** | `packages/ink-ui/src/lib/ink/rdp.js` | Ramer-Douglas-Peucker 단순화 (`ε = 0.5` 문서 px) |
| **순수 JS 엔진** | `packages/ink-ui/src/lib/ink/shapeFit.js` | ±4° 수평·수직 맞춤 + 0.5초 멈춤 도형 판정(직선 → 타원 → 사각형) |
| **순수 JS 엔진** | `packages/ink-ui/src/lib/ink/hitTest.js` | 지우개·고르기(`S`)용 점/선분 거리·bbox 충돌·모서리 핸들 판정 |
| **순수 JS 엔진** | `packages/ink-ui/src/lib/ink/inkEngine.js` | `InkEngine` 상태 기계(문서·도구·명령·Undo/Redo·스마트 우클릭·Esc 단계·고르기·보관/복원) — **DOM 0 의존** |
| **순수 JS 모델** | `packages/ink-ui/src/lib/ink/drawKeys.js` | 설정 `d-*` 29칸 → 키 매칭(`KeyboardEvent.code` 기반, IME `isComposing` 차단) |
| **순수 JS 렌더러** | `packages/ink-ui/src/lib/ink/canvasRender.js` | 2D Canvas 그리기 함수(굳힌 층 `bakedCanvas` + 활성 획 층 `activeCanvas` + 배경/모자이크 합성) |
| **Svelte 화면** | `packages/ink-ui/src/lib/components/DrawCanvas.svelte` | 포인터(`getCoalescedEvents`·필압)·휠·가장자리 밀기 rAF·글자 `T` 인라인 입력·펜 색 고리 커서·`MiniMap` |
| **Svelte 화면** | `packages/ink-ui/src/lib/screens/DrawOverlay.svelte` | `Alt+1`/`Alt+3` 전체 화면 컨테이너(`InkClient` 구독, 섬 동기화, `CheatSheet` `?` 토글, 복사·저장 트리거) |
| **Svelte 부품** | `packages/ink-ui/src/lib/components/Island.svelte` | 판서 섬 6색 펼침(0.8초)·4초 40% 물러남·커서 회피 속성 확장 |
| **Rust 도메인** | `crates/ink-domain/src/mode.rs` | `Mode::Draw { session, board, monitor_id, started_ms }` 추가 |
| **Rust 도메인** | `crates/ink-domain/src/modekeys.rs` | `DrawKeys` 추가(29칸 조회 — 필요한 경우) |
| **Rust 도메인** | `crates/ink-domain/src/zoom.rs` | 확대 단계·고정점 계산(판서·실시간 확대 공용, `Alt+2 → Alt+1` 이어받기용) |
| **Rust 유스케이스** | `crates/ink-app/src/draw.rs` | `DrawState`, `KeptInk`, 진입(`begin_draw`/`begin_board`/`begin_draw_restore`), 종료·보관·복원·복사·저장 |
| **Rust 어댑터** | `crates/tauri-plugin-screen-ink/src/lib.rs`, `overlay.rs` | 판서 명령 6개, `screen-ink://draw` 이벤트, 바이너리 합성 결과 수신 |
| **클라이언트** | `packages/ink-client/src/index.js`, `tauri.js`, `fake.js` | `DrawSession`, `KeptInkPayload`, `draw*` 메서드 6개 |

---

### 2.5 Rust 구조체 · 포트 · Tauri 명령 · `InkClient` 계약

#### Rust 도메인 / 앱 구조체

```rust
// crates/ink-domain/src/mode.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Mode {
    Idle,
    Capture { session: u64, kind: CaptureKind, started_ms: u64 },
    Draw { session: u64, board: bool, monitor_id: u32, started_ms: u64 },
    // 단계 5에서 LiveZoom, 단계 6에서 Recording(겹침 플래그) 확장
}
```

```rust
// crates/ink-app/src/state.rs
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawViewTransform {
    pub s: f64,   // 1.0 ~ 8.0
    pub tx: f64,  // CSS px (<= 0)
    pub ty: f64,  // CSS px (<= 0)
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawSession {
    pub session: u64,
    pub board: bool,                       // false = Alt+1 판서, true = Alt+3 칠판
    pub monitor: OverlayMonitor,           // 대상 모니터 1개(bounds, scale, frameUrl)
    pub layout_fingerprint: String,
    pub initial_view: DrawViewTransform,   // 기본 {s:1,tx:0,ty:0}, [판서로]·Alt+2 전환 시 확대값
    pub restore_on_open: bool,             // Alt+Shift+1일 때 true
    pub has_kept: bool,                    // Ctrl+Shift+R 힌트 표시 여부
    pub kept_compatible: KeptCompat,       // Same | Rescaled | DifferentMonitor | None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum KeptCompat { None, Same, Rescaled, DifferentMonitor }

// crates/ink-app/src/draw.rs — 메모리 보관(05 §7, 디스크에 쓰지 않음)
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeptInk {
    pub monitor_id: u32,
    pub layout_fingerprint: String,
    pub phys_w: u32,
    pub phys_h: u32,
    pub scale: f64,
    pub document: serde_json::Value,       // InkDocument JSON v1
}
```

#### Tauri 명령 & `InkClient` 메서드 (`plugin:screen-ink|<cmd>`)

| `InkClient` 메서드 | Tauri 명령 | 인자 → 반환 | 설명 |
|---|---|---|---|
| `drawReady(session, paintMs)` | `draw_ready` | `{ session, paintMs? } → void` | 진입 계측 `mode enter draw [N ms]`(M4) 기록 |
| `drawFinish(payload)` | `draw_finish` | `{ session, keep: boolean, kept?: KeptInk } → void` | 판서 종료, `keep && 항목≥1`이면 `keptInk` 저장 + 섬 `"판서를 보관했어요 · Alt+Shift+1"` |
| `drawRestore(session)` | `draw_restore` | `{ session } → { status: KeptCompat, kept?: KeptInk }` | `Ctrl+Shift+R` / `Alt+Shift+1` 복원 요청 |
| `drawExport(action, bytes, meta)` | `draw_export` | Raw IPC 바이트(`ArrayBuffer` / `Uint8Array`, 헤더로 `{ action: 'copy'\|'save', w, h }`) → `{ ok, name? }` | 합성된 RGBA/BGRA 원본 바이트를 base64 없이 전달(03 §4 원본 바이트 IPC) → `Clipboard` / `FileSink` |
| `drawIsland(patch)` | `draw_island` | `{ color, tool, widthPx, zoom, pageIndex?, pageCount?, idleDim?, notice? } → void` | 판서 섬(`ink-island`) 한 줄 상태 갱신(색·도구·굵기·배율·보정 안내) |
| `drawPenMetrics(sample)` | `draw_pen_metrics` | `{ n, p50, p95, max } → void` | M5 펜 지연 계측을 로그 파일에 1줄 기록(`pen latency n=.. p50=.. p95=.. ms`) |

- **이벤트**: `screen-ink://draw` (`DrawSession`), `screen-ink://state`, `screen-ink://island`.
- **오버레이 창 재사용**: `ink-overlay-<monitorId>` 창을 그대로 쓴다. `App.svelte`의 `view === 'overlay'`가 `state.mode.mode === 'draw'`이면 `DrawOverlay.svelte`, `'capture'`이면 `CaptureOverlay.svelte`를 마운트한다(추가 WebView2 프로세스 0개 → M3 대기 메모리 유지).
- **판서 진입 시 배경 표시**: `Alt+1`은 커서 모니터 1장만 `ScreenSource::capture`로 찍고, `DrawCanvas` 안의 배경 `<canvas>`(또는 `<img>`)로 그린다. 확대·이동 시 배경 이미지와 판서 층이 같은 `transform`으로 프레임 단위 동기화되어야 하므로, **판서 모드에서는 `nativeFrame` GDI 정지 창을 쓰지 않고 웹뷰 Canvas가 배경 이미지를 직접 그린다**. (1080p 1장 `ink://frame` 로드는 `createImageBitmap`으로 ~25~45 ms, 전체 진입 ≤ 120 ms 목표 M4 충족. 4K에서 150 ms 초과 시 §5 대응 적용.)

---

### 2.6 화면 상태 · 키 · 문구 (`04 S12·S14`)

- **도구 모드**: `'pen' | 'marker' | 'text' | 'select' | 'eraser' | 'pan'`
- **수정 키 임시 도형**(펜/형광펜 드래그 중):
  - `Shift` = 직선(`line`), `Ctrl` = 사각형(`rect`), `Ctrl+Shift` = 화살표(`arrow`), `Tab`(누른 채 드래그) = 타원(`ellipse`), `+Alt` = 점선(`dashed: true`).
  - 드래그 시작 뒤에도 수정 키를 누르거나 떼면 실시간으로 도형 미리보기가 바뀐다.
- **화면 안 키(`d-*` 29칸)**: `drawKeys.js`가 `InkState.shortcuts`에서 읽어 `KeyboardEvent.code`로 판정(`R-KEY-6`).
  - `d-pen`(P), `d-marker`(M), `d-text`(T), `d-select`(S), `d-eraser`(D), `d-clear`(E)
  - 색: `d-red`(R), `d-yellow`(Y), `d-green`(G), `d-orange`(O), `d-blue`(B), `d-lime`(L)
  - 배경(칠판): `d-white`(W), `d-black`(K) — `Alt+1` 판서 중 W/K를 누르면 즉시 흰/검은 칠판으로 전환, 다시 같은 키 또는 원래 화면 복귀? → `W`는 흰 칠판, `K`는 검은 칠판, `Alt+1` 판서 중 원래 정지 화면으로 돌아오는 규칙은 `W`/`K` 재입력 시 토글(`white → frame`, `black → frame`, 칠판 `Alt+3`에서는 `W`↔`K`).
  - 편집: `d-undo`(Ctrl+Z), `d-redo`(Ctrl+Y), `d-copy`(Ctrl+C), `d-save`(Ctrl+S), `d-restore`(Ctrl+Shift+R)
  - 보기: `d-center`(Space = 커서 가운데 + 1× 복귀), `d-zin`(Up), `d-zout`(Down), `d-wup`(Ctrl+Up), `d-wdown`(Ctrl+Down), `d-help`(Shift+/ = `?` 단축키 한 장 S16), `d-exit`(Esc)
  - `Ctrl+휠` = 굵기 1~30 조절, 일반 `휠` = 확대 단계 `1 → 1.25 → 1.5 → 1.75 → 2 → 3 → 4 → 6 → 8` (첫 휠은 `draw.startZoom`으로 점프).
- **스마트 우클릭 (`02 §3.4`, 설정 `draw.rightClickPan`)**:
  - `view.s === 1.0`: 우클릭 = 판서 끝내기(`drawFinish`).
  - `view.s > 1.0` & `draw.rightClickPan === true`:
    - 우클릭 누르고 떼기(또는 누름) = **손(이동) 모드(`pan`) 진입**. 마우스를 움직이는 만큼 `tx, ty` 이동(버튼을 누르지 않고 이동해도 따라가는 아이캔 줌잇 방식 + 우클릭 드래그 둘 다 자연스럽게 지원).
    - 손 모드에서 **좌클릭** = 직전 그리기 도구로 복귀(클릭 지점에서 획은 시작하지 않음 — 오작동 방지).
    - 손 모드에서 다시 우클릭 = 1×로 풀지 않고 유지, `Space`나 휠 축소로 1× 복귀.
- **가장자리 밀기 (`02 §3.4`)**:
  - `view.s > 1.0`이고 그리기/글자 입력 중이 아닐 때(또는 그리기 중 화면 밖으로 밀 때), 커서가 뷰포트 가장자리 `30 CSS px` 안에 들어오면 `rAF`마다 `12 CSS px / frame`(60fps 정규화: `720 px/sec × dt`)로 문서를 반대 방향 이동(`tx, ty` 가두기).
- **Esc 동작 (`draw.esc`)**:
  - `'step'`(기본): ① `?` 도움말 닫기 → ② 글자 입력 확정·닫기 → ③ 고르기(`select`) 해제 → ④ 손 모드(`pan`) 해제 → ⑤ 확대(`s > 1`)를 `1×`로 복귀 → ⑥ 판서 끝내기.
  - `'exit'`: 글자 입력 중이면 현재 글자를 커밋하고 즉시 판서 끝내기.
- **판서 섬 문구 (`04 S12·S14`, `10 §9`)**:
  - 진입: `판서 · 펜 4px` (보관 있으면 보조 `Ctrl+Shift+R 이전 판서`)
  - 칠판: `칠판 · 흰색 · 1 / 1` (검은색이면 `검은색`)
  - 보정: `수평으로 맞췄어요`, `직선으로 바꿨어요 · Ctrl+Z로 원래 획`, `원으로 바꿨어요 · Ctrl+Z로 원래 획`, `사각형으로 바꿨어요 · Ctrl+Z로 원래 획`
  - 복원: `이전 판서를 불러왔어요`, `배율이 바뀌어 크기를 맞췄어요`, `다른 모니터라 불러올 수 없어요`
  - 나가기: `판서를 보관했어요 · Alt+Shift+1`
  - 모두 지우기(`E`): `판서를 모두 지웠어요` + `[되돌리기]`

---

## 3. 알고리즘 · 결정 (대안과 고른 이유)

### 3.1 입력 파이프라인 (`getCoalescedEvents` + 필압)

```
pointerdown / pointermove
  └ e.getCoalescedEvents?.() ?? [e]   (WebView2 120~1000Hz 마우스/펜 중간 샘플 전부 수집)
  └ 각 샘플: (clientX, clientY, pressure, pointerType, timeStamp)
      - 마우스(pointerType === 'mouse' 또는 pressure === 0/0.5 고정): p = 0.5 (굵기 배수 1.0)
      - 펜(pointerType === 'pen'): widthMul = 0.6 + 0.8 * clamp(pressure, 0, 1)  (0.6x ~ 1.4x)
  └ viewToDoc(clientX, clientY, view) → 문서 좌표 (x, y)
  └ One Euro 필터 적용 (실시간, 0 지연 근사)
  └ 활성 획 버퍼에 추가 → rAF 내 activeCanvas에 마지막 세그먼트만 증분 렌더
pointerup
  └ 총 획 길이 < 6.0 문서 px (작은 한글 점·획): 필터·RDP 건너뛰고 원본 점 유지
  └ 아니면 RDP(ε = 0.5 문서 px)로 점 수 60~80% 절감
  └ Command::add(stroke) → bakedCanvas에 굳히기 → activeCanvas 비우기
```

### 3.2 떨림 보정 — One Euro 필터 (대안 비교)

| 후보 | 지연(M5 영향) | 작은 한글 획 | 구현 크기 | 판정 |
|---|---:|---|---|---|
| **One Euro 필터** (`02 §3.2`) | 매우 낮음(속도 비례 적응 컷오프) | `< 6px` 우회 + 빠른 획 무지연 | ~45줄 순수 JS, 의존성 0 | **채택** |
| `perfect-freehand` `streamline` | 중간(점 뒤따라감) | 뭉개짐 위험 | 외부 패키지 + 폴리곤 채움 | 보류(필요 시 굵기 윤곽만 참고) |
| `ink-stroke-modeler-rs` (Rust) | IPC 왕복(+3~8 ms) 추가 | 우수 | Rust↔JS 매 프레임 전송 부담 | 기각(지연 손해) |

**파라미터 (`02 §3.2` 확정값)**:
- `off`: 필터 미적용
- `light`(기본): `minCutoff = 1.0`, `beta = 0.007`, `dCutoff = 1.0`
- `strong`: `minCutoff = 0.6`, `beta = 0.004`, `dCutoff = 1.0`
- `dt = clamp((t_i - t_{i-1}) / 1000, 1/1000, 0.1)` (0 나눔 방지).

### 3.3 점 줄이기 — Ramer-Douglas-Peucker (`rdp.js`)

- 허용치 `ε = 0.5` 문서 px (`02 §3.2`).
- 조건: 획의 누적 경로 길이 `pathLen < 6.0` 문서 px이거나 점 개수 `<= 3`이면 **건너뛴다**(한글 자음 `ㅇ·ㅅ·ㅈ` 짧은 꺾임·마침표 보존).
- 필압 정보 `p`와 시각 `t`는 살아남은 원본 인덱스의 값을 그대로 유지한다.

### 3.4 도형 판정 (`shapeFit.js`)

1. **수평·수직 맞춤 (`draw.straighten`, 기본 켬)**:
   - 직선(`line`)·화살표(`arrow`) 끝점이 정해질 때 `angle = atan2(dy, dx)`.
   - `|angle|`이 `0°, 90°, 180°, -90°`에서 `±4°` 이내이면 `y2 = y1` 또는 `x2 = x1`로 스냅.
2. **0.5초 멈춤 자동 보정 (`draw.shapeOnHold`, 기본 켬)**:
   - 조건: `tool === 'pen'`(수정 키 없음), `pointermove` 중 마지막 기준점으로부터 이동 거리 `≤ 2.0` 문서 px(화면상 `≤ 2px`)인 상태가 **500 ms** 지속, 총 경로 길이 `≥ 24` 문서 px.
   - **판정 순서 (첫 통과 도형 채택, `02 §3.3`)**:
     1. **직선 (`line`)**: 시작점 `A`–끝점 `B` 직선과 모든 중간점의 최대 수직 거리 `maxDist < chordLen * 0.06` 이고 `chordLen ≥ 20`. (`±4°` 수평·수직 스냅도 함께 적용).
     2. **타원·원 (`ellipse`)**: 닫힘 조건 `dist(A, B) < max(bbox.w, bbox.h) * 0.25` (또는 경로 길이가 둘레의 85% 이상). 중심 `(cx, cy) = bbox 중심`, 반축 `(rx, ry) = (bbox.w/2, bbox.h/2)`. 각 점의 정규화 반지름 `r_i = hypot((x-cx)/rx, (y-cy)/ry)`에 대해 평균 절대 오차 `mean(|r_i - 1|) < 0.08` (8%). `|rx - ry| / max(rx, ry) < 0.12`이면 정원(`rx = ry = (rx+ry)/2`)으로 다듬음.
     3. **사각형 (`rect`)**: 닫힘 조건 만족 + RDP(`ε = 0.04 * diag`) 결과가 꼭짓점 **4개**(닫힌 점 제외 4~5점)이고, 4개 내각이 모두 `90° ± 12°`. 축 정렬 사각형(평균 기울기 `±8°` 이내면 수평 사각형으로 정렬).
     4. **실패**: 아무 변화 없음, 알림 없음.
   - **시각 피드백**: 0.5초 도달 시점에 즉시 도형으로 미리보기를 140 ms 스프링 보간(`10 §6`)하고 `pointerup` 때 `add(rawStroke)` + `correct(rawStroke, shape)` 2단계로 커밋. 섬에 `"직선으로 바꿨어요 · Ctrl+Z로 원래 획"` 표시. (주의: `04 S12`의 제안형 문구 대신 `02 §3.3`·`05 §6`의 즉시 변환 + `Ctrl+Z` 복원 방식을 채택 — 사용자 확인 목록 #2에 등재.)

### 3.5 렌더 구조 — 2층 Canvas + 굳히기 (OffscreenCanvas 검토)

| 층 | 요소 | 크기 | 언제 다시 그리나 |
|---|---|---|---|
| **0층 (배경)** | `<canvas class="ink-bg">` | `viewportW × dpr`, `viewportH × dpr` | 진입 1회, 확대·이동(`view` 변경), 칠판 `W`/`K` 전환 시 |
| **1층 (굳힌 판서)** | `<canvas class="ink-baked">` | 위와 동일 | 획 확정 시 **추가된 1획만 증분 draw**; Undo/Redo/Move/Delete/Zoom 변경 시 전체 재그리기 |
| **2층 (지금 획·선택 핸들)** | `<canvas class="ink-active">` | 위와 동일 | 포인터 이동 중 **현재 1획만** `clearRect(dirtyBbox)` + draw |
| **3층 (DOM 오버레이)** | 글자 `T` `<textarea>`, 커서 고리, `MiniMap` | CSS 배치 | 글자 입력 중 / 확대 중 |

- **왜 `OffscreenCanvas` + Web Worker를 1안으로 쓰지 않는가?**
  - WebView2에서 `CanvasRenderingContext2D` 메인 스레드 그리기는 활성 획 1개(점 50~300개) 기준 **0.3~1.2 ms**로 M5(50 ms) 대비 40배 여유가 있다.
  - Worker로 넘기면 `postMessage` 직렬화·동기화(특히 확대·글자 DOM 위치 맞춤·Undo 즉시 반응)가 복잡해지고, WebView2에서 `OffscreenCanvas` GPU 컨텍스트 손실 복구가 까다롭다.
  - **내보내기(복사·저장, `4.7`)**에는 `OffscreenCanvas`(미지원 시 숨은 `<canvas>`)를 1회성 합성 버퍼로 써서 물리 해상도(`physSize`)로 배경 + 전체 항목을 깨끗하게 합성한다.
- **곡선 보간**: 2차 베지어 중점 연결(`quadraticCurveTo(p[i], midpoint(p[i], p[i+1]))`)으로 꺾임 없이 매끄럽게 그린다. 필압 변화가 큰 펜(`pointerType === 'pen'`)은 세그먼트별 굵기 캡슐(또는 원 보간)로 가변 굵기를 표현한다.
- **WebView2 저지연 힌트**: `getContext('2d', { desynchronized: true, alpha: true })`를 시험 측정 후 채택(화면 찢어짐이 없으면 1프레임 ≈ 16 ms 절감, 찢어짐이 있으면 `desynchronized: false`로 전환).

### 3.6 성능 목표 M5 (펜 지연 p95 ≤ 50 ms) 측정법 및 미달 시 2안

- **내부 타임스탬프 계측(항상 가능)**:
  - `t_event = event.timeStamp` (OS 입력 시각과 동기화된 DOMHighResTimeStamp)
  - `t_paint = performance.now()` (해당 획을 `activeCanvas`에 그린 직후 `requestAnimationFrame` 콜백 끝)
  - 소프트웨어 지연 `latency_sw = t_paint - t_event`를 획 샘플마다 모아 세션 종료 시 `drawPenMetrics`로 로그 기록 (`pen latency n=.. p50=.. p95=.. ms`).
- **외부 실측(G5 관문, 학교 PC)**:
  - 스마트폰 240 fps 슬로우 모션 촬영: 검은 칠판(`Alt+3` → `K`)에서 마우스/펜을 빠르게 긋고, 커서 끝과 선 끝 사이의 프레임 수 × 4.17 ms로 end-to-end 지연 측정 (`n ≥ 20`).
- **미달 시 2안 (관문 G5 규칙, `07 §3`)**:
  1. 1단계 경량화(0.5일): `desynchronized: true` + `dirtyRect` 부분 `clearRect` + 1000Hz 마우스 샘플 4ms 데시메이션.
  2. 2단계 네이티브 렌더러(+2일): Windows `ink-platform-windows::draw_d2d`로 Direct2D + DirectComposition 투명 계층 창을 띄워, JS `inkEngine.js`의 상태 대신 `ink-domain::ink` Rust 포팅본(동일 JSON 스키마 v1)으로 네이티브 렌더.

### 3.7 순수 엔진(`inkEngine.js`) ↔ Svelte ↔ Rust 경계 & 묶음 B 공유 방법

```
┌─────────────────────────────────────────────────────────────┐
│  packages/ink-ui/src/lib/ink/  (순수 JS · DOM 0 · node --test) │
│   inkEngine.js  (문서 · 도구 · Undo/Redo · 보정 · 고르기)       │
│   canvasRender.js (2D Canvas 그리기 — 배경 + 항목 + 모자이크)    │
└───────────────┬─────────────────────────────┬───────────────┘
                │ 재사용 (100% 동일 엔진)       │
     ┌──────────▼──────────┐       ┌──────────▼──────────┐
     │ DrawOverlay.svelte  │       │ CaptureOverlay.svelte│
     │ (Alt+1 / Alt+3)     │       │ (4.9 캡처 주석 B)    │
     │ - 전체 모니터 문서   │       │ - 범위 rect 내부 문서 │
     │ - 휠 확대 · 손 모드  │       │ - 확대 없음 (s=1)     │
     │ - 보관 · 복원 · 칠판 │       │ - 모자이크 도구 활성  │
     └──────────┬──────────┘       └──────────┬──────────┘
                │ InkClient (draw*)            │ InkClient (capture*)
     ┌──────────▼─────────────────────────────▼──────────┐
     │  Rust (ink-app::draw / capture + tauri-plugin)    │
     │  - 단축키 진입 · 정지 프레임 캡처 · keptInk 보관    │
     │  - 클립보드(DIB+PNG) · 파일 저장 · 섬 알림          │
     └───────────────────────────────────────────────────┘
```

- **경계 원칙**:
  - **포인터 입력·획 보정·렌더·Undo/Redo**는 100% WebView 안의 `inkEngine.js` + `DrawCanvas`가 맡는다(매 포인터 이동마다 Rust IPC를 타지 않으므로 지연 최소화).
  - **Rust**는 ① 진입 시 정지 스냅숏 캡처·오버레이 띄우기, ② `keptInk` 메모리 보관/복원, ③ 최종 합성 이미지의 클립보드 복사/파일 저장, ④ 판서 섬(`ink-island`) 표시·전역 단축키·시스템 사건(잠금·모니터 변경) 종료만 맡는다.

---

## 4. 시험 · 합격 기준

### 자동 단위 시험 (`node --test`, `packages/ink-ui/src/lib/ink/*.test.js`)

| 시험 파일 | 대표 시험 이름 예시 | 검증 내용 |
|---|---|---|
| `coords.test.js` | `휠_확대_커서_고정점_유지_및_문서_가두기` | 1×→1.75×→4× 왕복 시 커서 아래 문서 좌표 오차 `< 1e-6`, 화면 밖 여백 0 |
| `oneEuro.test.js` | `원유로_보정_강도별_지연과_6px_미만_한글점_우회` | `off/light/strong` 지터 분산 감소, 4px 한글 점은 원본 좌표 그대로 |
| `rdp.test.js` | `RDP_직선_단순화_및_짧은획_보존` | 100점 직선 → 2점, `ε=0.5`, 5px 획 미단순화 |
| `shapeFit.test.js` | `수평_4도_맞춤_및_0_5초_멈춤_직선_원_사각형_판정` | 3.5°→0° 스냅, 5° 유지; 원 오차 5%→`ellipse`, 15%→실패; 사각형 4꼭짓점 90°±10°→`rect` |
| `inkEngine.test.js` | `보정_후_CtrlZ는_원래_손획으로_복귀` | `add` → `correct` → `undo`(원래 획) → `undo`(없음) |
| `inkEngine.test.js` | `확대_2배에서_4px는_문서_2px로_저장` | 화면 기준 굵기 확인(`R-DRAW-0`, `02 §3.2`) |
| `inkEngine.test.js` | `스마트_우클릭_1배는_종료_확대중은_손모드_좌클릭_복귀` | `view.s=1` 우클릭=`exit`, `view.s=2` 우클릭=`pan`, 좌클릭=이전 도구 |
| `inkEngine.test.js` | `글자_IME_조합중_도구키_무시_및_빈글자_미생성` | `isComposing=true`에서 `P`/`M`/`E` 무시, 공백만 입력 후 Esc 시 항목 0 |
| `inkEngine.test.js` | `실행취소_200단계_상한_및_지우개_한번끌기_단일명령` | 205회 추가 시 undo 길이 200, 지우개 3획 제거 후 Ctrl+Z 1회로 3획 복구 |
| `inkEngine.test.js` | `Esc_단계별_나가기_순서와_바로_나가기` | `step`: 도움말→글자→고르기→손→확대→종료 / `exit`: 즉시 종료 |
| `drawKeys.test.js` | `판서_화면안키_설정값_반영_및_맥_Cmd` | `d-pen`을 `A`로 바꾸면 `KeyA`=펜, `KeyP`=무시 |

### Rust 통합 시험 (`cargo test -p ink-app -p ink-platform-fake`)

- `draw_보관과_같은모니터_배율변경_다른모니터_복원`: `keptInk` 저장 → 같은 지문 `Same`, 배율 변경 `Rescaled`, 다른 모니터 `DifferentMonitor`.
- `alt_shift_1_직전판서_바로_열기`: `Mode::Draw` 진입 + `restore_on_open = true`.
- `잠금_모니터변경시_판서_안전종료_및_보관`: `SystemEvent::Lock` / `DisplayChange` → 자동 `keptInk` 보관 후 `Mode::Idle`.

### 관문 G5 · 실기 합격 기준

- **M4 진입**: `mode enter draw [N ms]` `p95 ≤ 150 ms` (학교 PC `n ≥ 30`).
- **M5 펜 지연(G5)**: 소프트웨어 `p95 ≤ 25 ms`, 240 fps 촬영 실측 `p95 ≤ 50 ms` (떨림 보정 `light` 켠 상태).
- **장면 1(문제 풀이)**: PPT 슬라이드 쇼 중 `Alt+1` → 휠 확대 → 빨간 펜 → `Esc` → `Alt+Shift+1` 복원 통과.

---

## 5. 위험 · 대응

| 위험 | 발생 신호 | 대응 |
|---|---|---|
| **M5 펜 지연 > 50 ms** (4K·저사양 교실 PC) | G5 실측 p95 초과 | `desynchronized: true` + dirty rect 부분 지우기 → 그래도 미달이면 Direct2D 네이티브 렌더러(+2일) |
| **4K 배경 이미지 로드 지연** (`Alt+1` 진입 > 150 ms) | `mode enter draw` > 150 ms | BMP 대신 raw BGRA `/raw-frame/<session>/<mon>` 엔드포인트로 `ImageData` 직접 업로드(BMP 인코딩·디코딩 생략), 또는 1× 상태에서는 기존 네이티브 `Freeze` 창(50% 어둡게 없이 원본)을 배경으로 두고 휠 확대 순간에만 웹뷰 캔버스로 전환 |
| **한글 IME WebView2 조합 깨짐** | `KeyP` 등이 `compositionend` 직후 1틱에 새어 나옴 | `e.isComposing || e.keyCode === 229` 이중 체크 + `compositionend` 뒤 `40 ms` 가드(macOS Safari/WKWebView `Enter` 확정 직후 이벤트 방지) |
| **확대 중 고르기(`S`)·글자(`T`) 좌표 오차** | 150% DPI + 3× 확대에서 텍스트 상자 어긋남 | 텍스트 `<textarea>` 위치·폰트 크기를 `docToView(item.x, item.y, view)`와 `item.fontSize * view.s`로 통일, `transform: none` |
| **판서 섬이 위쪽 획을 가림** | 커서가 상단 60px 진입 시 방해 | 커서가 섬 bbox + 24px 안에 들어오면 섬을 `translateY(-120%)` 또는 화면 아래쪽으로 회피(04 S12) |

---

## 6. 작업일 · 난이도 · PR 분할 구현 순서

- **작업일**: `3.00일` (`4.1`~`4.8` 각 0.375일) · **난이도**: **상** · **선행**: 단계 2 완료(끝남).

### 첫 PR 단위로 쪼갠 구현 순서

| PR | 범위 (`progress.json`) | 생성/수정 파일 | 통과할 시험 |
|---:|---|---|---|
| **PR-A1** | `4.1` 순수 엔진 코어 | `packages/ink-ui/src/lib/ink/{coords,oneEuro,rdp,shapeFit,hitTest,inkEngine,drawKeys}.js` + 각 `*.test.js` | `npm test` (엔진 단위 시험 35+개) |
| **PR-A2** | `4.2` 렌더러 + Rust 판서 모드 진입 + M5 계측 | `canvasRender.js`, `DrawCanvas.svelte`, `DrawOverlay.svelte`, `ink-domain/src/mode.rs`·`zoom.rs`, `ink-app/src/draw.rs`, `tauri-plugin-screen-ink`, `packages/ink-client`, `apps/screen-ink/src/App.svelte` | `cargo test`, `npm run check`, 학교 PC M4·M5(G5) 측정 |
| **PR-A3** | `4.3`·`4.4` 키 29칸·칠판·확대·가장자리 밀기·스마트 우클릭 | `DrawOverlay.svelte`, `DrawCanvas.svelte`, `inkEngine.js` 보강 | 키 29칸 표 시험, 확대·스마트 우클릭 단위+수동 시험 |
| **PR-A4** | `4.5` 글자 T(IME)·고르기 S·다시 고치기 | `DrawCanvas.svelte`(인라인 텍스트 편집기·핸들), `inkEngine.js` | 한글 IME 조합 보호 시험, 이동·크기·속성 Undo 시험 |
| **PR-A5** | `4.6`·`4.7` 보관·복원·복사·저장 | `ink-app/src/draw.rs`(`keptInk`·`draw_export`), `tauri-plugin-screen-ink`(원본 바이트 IPC) | 보관·배율 복원·다른 모니터 거부·PNG 복사/저장 통합 시험 |
| **PR-A6** | `4.8` 판서 섬(6색 펼침·물러남)·펜 고리 커서·MiniMap·칠판 미리보기 | `Island.svelte`, `IslandHost.svelte`, `DrawOverlay.svelte`, `preview/Preview.svelte` | F13·F15 시안 대조, `MiniMap` 비율 단위 시험 |
