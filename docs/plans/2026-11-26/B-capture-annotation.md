# B. 캡처 주석 · 결과 섬 HEX 칩 · [판서로] 이어받기 구현 시안 (`4.9` · `3e-chip` · `3e-handoff`)

대상 항목(`progress.json`): `4.9` 캡처 주석(`R-CAP-28`, 1.0일) · `3e-chip` 결과 섬 HEX 칩·주황 잔상 0.4초(`R-CAP-14`·`R-CAP-26`, 0.15일) · `3e-handoff` `[판서로]` 이어받기 ≤ 150 ms(`R-CAP-26`, 0.2일).

---

## 1. 목표 · 범위 (요구사항 ID)

| 요구사항 | 우선 | 요약 |
|---|---|---|
| `R-CAP-28` | P0 | 선택 툴바(`CaptureToolbar`)에 주석 도구 8종(사각형·타원·화살표·펜·형광펜·모자이크·글자·지우개) + 실행 취소/다시 실행 추가. **A의 `inkEngine.js`를 그대로 재사용**, 확정 범위 안에서만 그림, 복사·저장·핀(`R-CAP-29`) 결과에 합성 |
| `R-CAP-27` | P0 | 선택 툴바 전체 구성 완성(`04 S08`): `[주석 8종 │ 실행 취소·다시 실행 │ 취소·고정·저장·복사(주황)]` |
| `R-CAP-14` | P0 | 결과 섬 단색 추천 칩 `[#RRGGBB 복사]` 표시 및 클릭 동작(백엔드는 `IslandEvent.chip` 이미 전송 중, UI 표시만 누락) |
| `R-CAP-26` | P0 | ① 확정 직후 고른 자리에 **주황 잔상 0.4초** 표시 ② 결과 섬 `[판서로]` 단추 클릭 → **방금 범위를 확대한 채 `Alt+1` 판서 진입 ≤ 150 ms** |

---

## 2. 정확한 형식

### 2.1 `4.9` 캡처 주석 — 데이터 · 모듈 · 툴바 구조

#### 툴바 항목 정의 (`packages/ink-ui/src/lib/models/captureToolbar.js`)

기존 `TOOLBAR_ACTIONS`를 세 묶음(구분선 `divider: true`)으로 확장한다.

```js
export const TOOLBAR_ANNOTATE_TOOLS = Object.freeze([
  { id: 'rect',      icon: 'rect',      label: '사각형',   tool: 'shape',  shape: 'rect' },
  { id: 'ellipse',   icon: 'ellipse',   label: '타원',     tool: 'shape',  shape: 'ellipse' },
  { id: 'arrow',     icon: 'arrow',     label: '화살표',   tool: 'shape',  shape: 'arrow' },
  { id: 'pen',       icon: 'pen',       label: '펜',       tool: 'pen' },
  { id: 'marker',    icon: 'marker',    label: '형광펜',   tool: 'marker' },
  { id: 'mosaic',    icon: 'mosaic',    label: '모자이크', tool: 'mosaic' },
  { id: 'text',      icon: 'text',      label: '글자',     tool: 'text' },
  { id: 'eraser',    icon: 'eraser',    label: '지우개',   tool: 'eraser' },
]);

export const TOOLBAR_HISTORY = Object.freeze([
  { id: 'undo', icon: 'undo', label: '실행 취소', slot: 'd-undo', command: 'annot.undo' },
  { id: 'redo', icon: 'redo', label: '다시 실행', slot: 'd-redo', command: 'annot.redo' },
]);
// 뒤이어 기존 TOOLBAR_ACTIONS: cancel | pin | save | copy(primary)
```

- **보조 옵션 바(2단 작은 알약, 도구를 고른 상태에서만 툴바 위/아래 4px에 표시)**:
  - 펜·형광펜·도형·글자 선택 시: **6색 점(R·Y·G·O·B·L)** + **굵기 3단(가는 2px · 보통 4px · 굵은 8px, `Ctrl+휠`로 미세 조절)**.
  - 모자이크 선택 시: **블록 크기 3단(작게 6px · 보통 10px · 크게 16px)**.
  - 도구를 다시 누르면 선택 해제 → 원래 범위 조정(방향키·핸들 끌기) 상태로 복귀(`04 S08`).

#### 주석 엔진 인스턴스 & 좌표 경계

- `CaptureOverlay.svelte`는 `captureView.phase === 'confirmed'`일 때 `CaptureAnnotator.svelte`를 마운트한다.
- **좌표**: 확정 범위 `rect: PhysRect`(가상 화면 물리 px)를 기준으로,
  - 문서 원점 `(0, 0)` = `(rect.x, rect.y)`
  - 문서 크기 `(docW, docH)` = `(rect.w / scale, rect.h / scale)`
  - 줌 고정 `view = { s: 1, tx: 0, ty: 0 }`
  - `CaptureAnnotator` `<canvas>`는 CSS `left: frame.x`, `top: frame.y`, `width: frame.width`, `height: frame.height`, `overflow: hidden`(범위 밖 자동 클리핑).
- **여러 모니터에 걸친 범위**:
  - 주석 데이터(`InkDocument` 항목)는 **가상 화면 물리 px 원점 `(rect.x, rect.y)` 기준 단일 문서**로 툴바 소유 모니터(`toolbarOwner`)가 주 편집을 맡는다.
  - 여러 모니터에 걸친 범위에서 다른 모니터 영역까지 그려야 할 경우: `InkClient.captureAnnotSync(session, docPatch)`로 같은 세션의 모든 오버레이 창에 문서 JSON을 브로드캐스트해 각 모니터가 자기 구간(`offset = monitor.bounds - rect.origin`)만 클립해 그린다.

#### 합성 결과 전달 계약 (UI → Rust)

주석이 **0개**이면 기존 경로(`CaptureInput::Command { name: "copy" | "save" | "pin" }`)를 그대로 탄다(성능·호환 0 손해).
주석이 **1개 이상**이면 UI가 확정 시점에 **주석 레이어만(투명 RGBA) 또는 배경+주석 합성 버퍼**를 Rust로 보낸다.

**선택한 방식: 주석 전용 투명 RGBA 바이트 전송 → Rust에서 원본 `frames`와 알파 합성 + 모자이크 적용**
- 이유:
  1. `nativeFrame=true`(Windows)에서 웹뷰가 아직 배경 이미지 로딩을 끝내지 않았더라도, 모자이크가 없는 주석(펜·도형·글자)은 **배경 없이 투명 캔버스만** 물리 크기 `(rect.w, rect.h)`로 뽑아 Rust에 보내면 즉시 합성된다.
  2. 모자이크 항목(`kind: 'mosaic'`)은 **좌표 목록 `Vec<MosaicRect>`**만 JSON으로 함께 보내 Rust(`ink_domain::image::apply_mosaic`)가 원본 BGRA 버퍼에 직접 블록 평균을 적용한 뒤, 그 위에 주석 RGBA 레이어를 `source-over` 합성한다 → 100% 무손실, 여러 모니터 혼합 배율 경계에서도 오차 0 px.

```rust
// crates/ink-domain/src/image.rs 추가
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MosaicRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub block: u32, // 물리 px 단위 블록 크기(최소 4, 최대 64)
}

/// 원본 BGRA 버퍼에 모자이크 블록 평균 적용 (순수 함수)
pub fn apply_mosaic(bgra: &mut [u8], w: u32, h: u32, rects: &[MosaicRect]);
/// 투명 RGBA(premultiplied 또는 straight) 주석 레이어를 BGRA 원본 위에 알파 합성 (순수 함수)
pub fn composite_rgba_over_bgra(dst_bgra: &mut [u8], src_rgba: &[u8], w: u32, h: u32);
```

- **Tauri 명령 & `InkClient` 메서드**:
  - `captureCommitAnnotated(session, action, rect, mosaics, overlayRgbaBytes)` → `plugin:screen-ink|capture_commit_annotated` (원본 바이트 IPC `tauri::ipc::Request::body()` + JSON 헤더 `x-ink-meta`).
  - `action`: `'copy' | 'save' | 'quickSave' | 'pin'`.
  - Rust `InkService::finish_annotated(rect, action, mosaics, overlay_rgba)`가 ① `render_rect(rect)` → ② `apply_mosaic` → ③ `composite_rgba_over_bgra` → ④ 복사/저장/핀(`R-CAP-29`에도 주석 자동 반영!)을 수행한다.

---

### 2.2 `3e-chip` 결과 섬 HEX 칩 & 주황 잔상 0.4초

#### ① 결과 섬 추천 칩 (`Island.svelte` + `IslandHost.svelte`)

백엔드 `crates/ink-app/src/capture.rs:500`은 이미 `IslandEvent.chip = Some(IslandButton { label: "#RRGGBB 복사", action: "capture.copyHex" })`를 보내고 있다. `Island.svelte`와 `IslandHost.svelte`만 연결하면 끝난다.

- `packages/ink-client/src/index.js`: `IslandEvent` JSDoc에 `@property {{label: string, action: string}} [chip]` 추가.
- `packages/ink-ui/src/lib/components/Island.svelte`:
  - props에 `chip?: { label: string, hex?: string, onClick?: () => void } | null` 추가.
  - `secondaryButton` 옆(또는 `button` 다음)에 작은 알약 칩 렌더링: 왼쪽 10×10px 둥근 색 견본(`background: hex`, 1px 흰 테두리) + 고정폭 글자 `#RRGGBB 복사`.
- `packages/ink-ui/src/lib/screens/IslandHost.svelte`:
  - `current.chip`을 `{ label, hex: extractHex(current.chip.label), onClick: () => client.run(current.chip.action) }`로 바인딩.

#### ② 고른 자리 주황 잔상 0.4초 (`R-CAP-26`, `04 S08`, `10 §6`)

현재 문제(`docs/handoff/2026-11-25-mac-windows.md §2.3 X2`): `SelectionFrame.svelte`에 `state="afterimage"` CSS 애니메이션(`400ms linear`)은 이미 있지만, 확정 직후 `end_capture()`가 오버레이 창을 즉시 숨겨서 화면에 보이지 않는다.

**정확한 구현 형식**:
- `OverlayHost`에 `flash_afterimage(&mut self, rect: PhysRect, layout: &MonitorLayout)` 메서드를 추가한다(또는 기존 `ink-overlay-<id>` 웹뷰를 400 ms만 클릭 통과 상태로 남겨 두고 잔상만 그림).
- **채택안(창 추가 0개, M1 진입 영향 0)**:
  1. 확정 순간 `end_capture()`에서 **네이티브 정지 화면(`Freeze`)은 즉시 숨기고**(`hide_all`), `ink-overlay-<id>` 웹뷰에 `screen-ink://afterimage { rect, durationMs: 400 }` 이벤트를 보낸다.
  2. 해당 모니터의 오버레이 창을 OS 레벨 **클릭 통과(`WS_EX_TRANSPARENT` / `ignoresMouseEvents = true`)** + 비포커스 상태로 전환하고, `CaptureOverlay.svelte`가 `SelectionFrame state="afterimage"`만 투명 바탕 위에 그린다.
  3. 400 ms 타이머(또는 새 세션 시작 시 즉시 취소) 후 오버레이 창을 숨기고 클릭 통과를 원래대로 되돌린다.
  4. `prefers-reduced-motion: reduce`이면 잔상을 건너뛰고 즉시 숨긴다(`10 §6`).

---

### 2.3 `3e-handoff` 결과 섬 `[판서로]` 이어받기 (≤ 150 ms)

- **보관 데이터 확장 (`crates/ink-app/src/capture.rs`)**:
  ```rust
  pub(crate) struct LastCapture {
      pub w: u32,
      pub h: u32,
      pub bgra: Arc<Vec<u8>>,
      pub app: Option<String>,
      pub hex: Option<String>,
      // 3e-handoff 추가:
      pub rect: PhysRect,                  // 방금 고른 범위(가상 화면 물리 px)
      pub monitor_id: u32,                 // 범위 중심이 속한 모니터
      pub monitor_bounds: PhysRect,
      pub monitor_scale: f64,
      pub monitor_frame: Option<Frame>,    // 방금 캡처했던 그 모니터 전체 정지 이미지(Arc — 0 복사)
      pub layout_fingerprint: String,
  }
  ```
- **메모리 관리**: `monitor_frame`은 결과 섬이 떠 있는 동안(또는 다음 캡처/30초 유휴 타이머)만 유지하고, 30초 뒤 자동 `None`으로 해제해 M3 대기 메모리(≤ 150 MB)를 지킨다.
- **결과 섬 단추 (`capture.rs:495`)**:
  - 주 단추: `[저장]` (`capture.saveLast`, `Ctrl+S`)
  - 보조 단추(`secondaryButton`): `[판서로]` (`capture.toDraw`, `Alt+1`)
  - `IslandEvent`에 `secondary_button: Option<IslandButton>`, `secondary_button_key: Option<String>` 추가.
- **`capture.toDraw` 동작 (목표 ≤ 150 ms)**:
  1. `LastCapture`의 `monitor_frame`이 살아 있고 현재 `MonitorLayout.fingerprint()`가 같으면 **화면을 다시 캡처하지 않고** 기존 `monitor_frame`을 그대로 판서 배경으로 재사용(캡처 시간 **0 ms**!).
  2. 초기 확대 변환(`DrawViewTransform`) 계산 (`ink_domain::zoom::fit_rect_view`):
     - `rect`를 대상 모니터 뷰포트의 88% 안에 꽉 차게 맞추는 배율 `s = clamp(min(0.88 * docW / selDocW, 0.88 * docH / selDocH), 1.0, 8.0)`.
     - 범위 중심이 모니터 화면 가운데에 오도록 `tx, ty` 계산 후 문서 경계 안으로 가두기.
  3. `begin_draw_with_frame(monitor_id, monitor_frame, initial_view)` 호출 → `DrawSession` 발송 → 오버레이 즉시 표시.
  4. 만약 모니터 구성이 바뀌었거나 30초 지나 프레임이 해제됐으면 새로 캡처한 뒤 같은 배율 공식으로 진입.

---

## 3. 알고리즘 · 결정 (대안과 고른 이유)

| 결정 지점 | 고른 안 | 대안 | 고른 이유 |
|---|---|---|---|
| 주석 합성 주체 | **모자이크 = Rust(원본 BGRA), 선·도형·글자 = UI 투명 RGBA → Rust 합성** | UI가 전체 배경 `<img>`를 그려서 PNG 추출 | `nativeFrame=true`에서 배경 로드 대기 없음, 혼합 배율·다중 모니터 경계 0 px 오차, 기존 `render_rect`·커서 합성·캡처 핀(`R-CAP-29`) 파이프라인에 자연 통합 |
| 모자이크 미리보기 | UI `CaptureAnnotator`가 `offscreenCanvas`(확대경용으로 이미 디코딩된 배경)에서 블록 평균으로 즉시 렌더, 아직 로딩 전이면 반투명 격자 자리표시자 후 로드 완료 시 자동 갱신 | 매 드래그마다 Rust에 픽셀 질의 | 60 fps 지연 없는 드래그 경험 + Rust IPC 부하 0 |
| 잔상 0.4초 창 | 기존 `ink-overlay-<id>`를 400 ms만 클릭 통과로 유지 | 별도 잔상 전용 창 생성 | 창/WebView2 추가 메모리 0 MB, 새 F1 즉시 진입 시 잔상만 0 ms에 취소 |
| `[판서로]` 프레임 | `LastCapture.monitor_frame` 30초 캐시 재사용 | `[판서로]` 클릭 시 재캡처 | 150 ms 목표를 ~40 ms로 여유 있게 통과 + 결과 섬이 떠 있던 순간의 장면이 변하지 않음 |

---

## 4. 시험 · 합격 기준

| 시험 종류 | 시험 이름 예시 | 합격 기준 |
|---|---|---|
| Rust 단위 (`ink-domain`) | `모자이크_블록_평균_및_경계_클리핑` | 지정 사각형 안만 블록 단색 평균, 바깥 픽셀 0 변화 |
| Rust 단위 (`ink-domain`) | `투명_주석_RGBA_원본_BGRA_알파합성` | 반투명 형광펜·불투명 빨간 펜 색 정확도 오차 ≤ 1/255 |
| Rust 단위 (`ink-domain`) | `범위_이어받기_초기확대_계산` | 400×200 범위를 1920×1080에 중앙 88% 맞춤, 배율 `[1, 8]` 가두기 |
| Rust 통합 (`ink-platform-fake`) | `주석_합성후_복사_저장_및_핀_결과에_반영` | 클립보드·저장 PNG·핀 이미지 모두 주석 픽셀 포함 |
| Rust 통합 (`ink-platform-fake`) | `결과섬_판서로_이어받기_재캡처없이_진입` | `ScreenSource::capture` 호출 0회 증가, `DrawSession.initial_view.s > 1.0` |
| JS 단위 (`node --test`) | `툴바_주석도구_토글_및_보조팔레트_배치` | 도구 재클릭 시 선택 해제, 툴바 화면 밖 넘침 0 |
| JS 단위 (`node --test`) | `결과섬_HEX칩_추출_및_클릭액션` | `"#FF9F0A 복사"` → 견본 `#FF9F0A`, `client.run('capture.copyHex')` 호출 |
| 실측 | `[판서로]` 진입 시간 | `mode enter draw [N ms]` **≤ 150 ms** (`n ≥ 20`) |
| 눈 검사 | 주황 잔상 0.4초 + 확정 직후 다른 앱 클릭 | 잔상 표시 중에도 아래 앱 클릭이 즉시 통과됨 |

---

## 5. 위험 · 대응

| 위험 | 발생 신호 | 대응 |
|---|---|---|
| **주석 그린 뒤 범위를 방향키로 이동/크기 변경** | 주석 위치가 범위와 어긋남 | 도구를 해제하고 범위를 바꿀 때 주석 원점 `(rect.x, rect.y)` 대비 이동량 `(dx, dy)`만큼 문서 좌표를 보정하거나, 범위 크기 변경 시 주석 클리핑 유지(문서 원점은 가상 화면 물리 좌표 기준으로 보관해 범위만 늘려도 주석이 화면과 같은 자리에 남게 함) |
| **결과 섬 폭 520 px 초과** (`썸네일 + 복사했어요 + [저장] + [판서로] + [HEX 칩]`) | 버튼이 잘리거나 두 줄로 깨짐 | 3개 액션 동시 존재 시 단추 안 키캡(`Ctrl S`·`Alt 1`)을 자동 생략해 폭을 480 px 이하로 유지 |
| **잔상 400 ms 도중 사용자가 즉시 F1 재진입** | 이전 잔상 타이머가 새 오버레이를 숨김 | `session` 번호로 가드: `afterimage` 타이머 콜백은 `currentSession === afterimageSession`일 때만 `hide()` |
| **30초 프레임 캐시 메모리** | 4K×2 모니터 = ~64 MB 일시 점유 | 대상 모니터 **1장만** 보유(4K ≈ 33 MB, FHD ≈ 8 MB), 30초 타이머 + 새 캡처/모드 진입 시 즉시 해제 |

---

## 6. 작업일 · 난이도 · 먼저 끝나야 할 것

| 항목 | 작업일 | 난이도 | 먼저 끝나야 할 것 |
|---|---:|---|---|
| `3e-chip` (HEX 칩 + 주황 잔상 0.4초) | 0.15일 | 하 | 없음 (지금 바로 가능) |
| `4.9` 캡처 주석 (`R-CAP-28`) | 1.00일 | 상 | **A**의 `4.1` `inkEngine.js` · `4.2` `canvasRender.js` |
| `3e-handoff` (`[판서로]` 이어받기) | 0.20일 | 중 | **A**의 `4.2`·`4.4`·`4.6` (`DrawSession` 진입·초기 확대) |
| **묶음 합계** | **1.35일** | **상** | **A 완료 후 4.9·3e-handoff 착수** |
