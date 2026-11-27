# 인수인계 — 2026-11-26 마감 (Windows 병렬 작업 → 맥에서 이어서)

- 기준: 브랜치 `feat/foundation`, 이 문서를 넣은 커밋(맨 아래 §8에 해시). 진척 = [docs/progress/README.md](../progress/README.md).
- **이 문서가 맥 작업 지시서다.** 이전 문서 [2026-11-25-mac-windows.md](2026-11-25-mac-windows.md)의 §1 공통 규칙·§5 파일 지도는 그대로 유효하고, 바뀐 것은 아래 §1에 적었다.
- 오늘 Windows에서는 메인 세션이 일을 흐름 11개로 쪼개 Claude 일꾼·제미나이(antigravity) 네온에게 동시에 맡기고 병합했다. 운영 기록 = [docs/plans/work-streams.md](../plans/work-streams.md), 설계 시안 = [docs/plans/2026-11-26/](../plans/2026-11-26/index.md), 실측 = [spikes-windows.md](../plans/2026-11-26/spikes-windows.md).
- 표기: **[완료]** 코드·자동 시험 끝 · **[확인 남음]** 실제 화면 확인만 남음 · **[할 일]** · **[확인 필요]** 아직 사실 확인 안 한 가정

> **2026-11-27 낮 — 2차 고도화(마스터 플랜 [PRD-2차](../plans/2026-11-27/PRD-2차-고도화.md) · 진척 표 `r2` 트랙 60%)**
> - 들어간 것: 단축키 녹화 버그 · 다국어 틀(영역 파일 `i18n/areas` → `npm run i18n`, OS별 `~mac`·`~win`) · 화면 1,243키·Rust 201키 영어·쉬운 말 · 맞춤 아이콘 46종 · 캡처 툴바·색 16·굵기 슬라이더·스포이트 · 글꼴 Freesentation·Paperlogy(크기 보정, 627 KiB) · 트레이 재디자인 · 첫 실행 카드 5장·설정 점검·사용 설명서·설정 찾기 · 그리기 도구 막대 · 글자 글꼴 고르기 · 단축키 PDF(pdf-lib) · 맥 자동 업데이트(OS 도구 + 같은 인증서 확인, `scripts/release-mac.sh`) · 가림 인지 캡처 · 트레이 전체 화면 대응(계측 줄 `tray click: …`).
> - 분업: 화면·잡무는 제미나이 G1~G11(브리지), 핵심·검수는 Claude. 갈래 보고는 PRD §11 표.
> - **사람 확인 남음**(Claude의 키·마우스 자동 입력 시험이 승인 분류기에 막혀 못 함): ① 맥 캡처 글 상자 한글 입력(R2-TXT-0) — 개발 실행에서 글 상자를 누르면 앱 기록에 `input state [annot-text] …` 한 줄이 남는다(키 창·첫 응답자·입력 문맥·입력 소스). 그 줄과 `[ime] keydown`이 오는지로 원인을 가린다. ② 트레이 전체 화면(설정 창을 연 채 전체 화면 앱에서 메뉴 막대 클릭 → 기록 `tray click: show (… app active a->b …)`). ③ 새 화면들 눈 확인·PDF 저장·영어 전환·업데이트(릴리스 게시 뒤).

> **2026-11-27 맥 세션 결과 — M13 실시간 확대 + 확대 고도화(사용자 요청)**
> - 맥 돋보기 `ink-platform-macos/src/magnifier.rs`·`wheel.rs` + HUD(`zoom_hud.rs` `mac`): Option+2 진입 49~156 ms, 틱 58 Hz, 우리 창 제외, Esc·Alt+2로 끝, 끝난 뒤 커서 복구 — 내 셸에서 띄운 개발 앱 실기.
> - 공통 고도화(사용자 요청 "더 부드럽게·0.25 단계는 성김·1배까지·Esc로 끄기·마우스 위치 따라 확대 지점"): §2.1 **M13 결과**.
> - 남은 맥 일(순서): ① 사용자 눈 확인(확대 느낌·트랙패드) ② 캡처 중 글 상자 한글 조합 버그(§2.1 M16) ③ 서명한 `.app`에서 M9·M10·M13·M14·M15 실기(인증서는 사람이 만듦, `mac-self-sign.md`) ④ 귀 투명 확인(M14) ⑤ §2.4 판서 엔진 확장·여러 모니터 주석 동기화.
>
> **2026-11-26 밤 맥 세션 결과(요약 — 자세한 것은 §2.1 각 결과)**
> - 실기 확인: M12 판서·칠판 · M16 캡처 주석·핀·칩·잔상 · 원본 바이트 IPC(WKWebView 확인됨). 셸 실측: M9 AX 색인 48개 42 ms · M14 F2 창 48~161 ms · M15 녹화 4K 2초 47프레임 moov.
> - 새로 만든 것: M9 `ax.rs` · M10 권한 스마트 흐름 · M14 `window_shot.rs` · M15 `recorder.rs` · M17 F키 한 줄 · M-sign 스크립트·안내.
> - 고친 버그: 판서 뒤 커서 사라짐 · 옛 파일 이름 틀 · 맥 휠 한 칸 · 결과 섬 0.2초 사라짐 · 핀 혼합 배율 · 맥 "Ctrl+V" 표기 · 트레이 확대가 판서를 안 끝냄 · 결과 섬 중 Alt+1 = [판서로].
> - 남은 맥 일(순서): ① 캡처 중 글 상자 한글 조합 버그(§2.1 M16) ② M13 실시간 확대(§2.1 설계 메모) ③ 서명한 `.app`에서 M9·M10·M14·M15 실기(인증서는 사람이 만듦, `mac-self-sign.md`) ④ 귀 투명 확인(M14) ⑤ §2.4 판서 엔진 확장·여러 모니터 주석 동기화.
> - 개발 실행(`npm run app:dev`)은 권한이 터미널 앱 몫이다 — 손쉬운 사용이 없으면 A 근거·휠 가로채기가 안 된다.

---

## 0. 한눈에 — 오늘 들어온 것

| 영역 | 상태 | 핵심 파일 | 남은 확인 |
|---|---|---|---|
| 판서 엔진 4.1 | [완료] 시험 69+ | `packages/ink-ui/src/lib/ink/`(inkEngine·coords·oneEuro·rdp·shapeFit·hitTest·drawKeys·canvasRender·penMetrics) | — |
| 판서 모드 4.2~4.7 (Alt+1 판서·Alt+3 칠판·보관·복원·복사·저장) | [확인 남음] | `components/DrawCanvas.svelte`, `screens/DrawOverlay.svelte`, `crates/ink-app/src/draw.rs`, 플러그인 `draw_*` 명령 6개 | G5 펜 지연(학교 PC), 4K 내보내기 멈칫 |
| 판서 화면 4.8 (섬 6색·4초 물러남·커서 비킴·펜 색 고리·MiniMap·칠판 미리 보기) | [확인 남음] | `models/drawUi.js`, `Island.svelte`, `IslandHost.svelte`, `MiniMap.svelte`, `DrawZoomPage.svelte` | 미리보기 `?draw=1` |
| 캡처 주석 4.9 (도구 8종·실행 취소, Rust 모자이크+알파 합성) | [확인 남음] | `components/AnnotateLayer.svelte`, `models/captureAnnotate.js`, `ink-domain/src/image.rs`, `capture.rs` `commit()` | 툴바 넘침·한글 IME·150% |
| 결과 섬 HEX 칩·주황 잔상 0.4초 (3e-chip) | [확인 남음] | `Island.svelte`, `overlay.rs` `flash_afterimage`(화면 이벤트 방식, eval 없음) | 잔상 중 클릭 통과 |
| 이어받기 5.3(확대 중 Alt+1)·3e-handoff([판서로]) | [확인 남음] | `ink-app/src/{live_zoom,draw,capture}.rs`, `ink-domain/src/zoom.rs` | ≤150 ms 실측 |
| 실시간 확대 5.1·5.2·5.5 (Windows) | [확인 남음] | `ink-platform-windows/src/magnifier.rs`, `ink-app/src/live_zoom.rs`, `ink-domain/src/zoom.rs`, 플러그인 `zoom_hud.rs`, `ZoomHud.svelte` | M6 fps(`bench/m6-zoom.md`) |
| F2 창 캡처(WGC, 둥근 귀 투명 RGBA)·바로 캡처 4종(지연은 카운트다운 중 Esc 취소)·F3 녹화(MF 5 Mbps, 녹화 중 Alt+1·Alt+2는 녹화를 끊지 않음)·자동 중지·복구·빨간 꺾쇠 (6.1~6.5, Windows) | [확인 남음] · 마이크는 자리만 | `ink-platform-windows/src/{gpu,wgc_window,recorder}.rs`, `ink-app/src/record.rs`, `ink-domain/src/recording.rs`, 플러그인 `rec_frame.rs`, `RecordFrame.svelte` | 테두리 육안·10분 녹화 |
| 인식 마무리 (3b-dpi·3b-msaa·RB-15/16·시험 앱·Snipaste 판독) | [확인 남음] | `ink-platform-windows/src/{dpi,msaa}.rs`, `tests/rb15_16.rs`, `bench/recog/`(RUNBOOK-windows.md) | 화면 절차 1.7~2시간 |
| MSIX 포장 D2·패키지 모드 D3 | [완료]·실설치 확인 남음 | `scripts/package-msix.ps1`, `apps/screen-ink/src-tauri/packaging/msix/`, `ink-domain/src/package.rs`, `ink-platform-windows/src/package.rs`, `.github/workflows/release-msix.yml` | 자체 서명 설치(사용자 승인) |
| CI 상시 실행 | [완료] | `.github/workflows/ci.yml`(푸시마다 ubuntu·Windows·맥·UI, Windows 순수 판정 시험 추가) | — |
| 앱 이름·아이콘 (사용자 브랜드 세션) | [완료] | 커밋 8796250·848d271, `docs/brand/README.md` | — |

**맥 어댑터는 오늘 Windows 흐름에서 바뀌지 않았다**(아침 맥 커밋 `621e070`~`84723a9` — 진입 p50 64 ms·모니터 넘어 끌기·섬 위치 — 은 이전 문서 §2.5에 있다). 다만 공통 코드가 크게 늘어 맥에서 확인할 것이 생겼다(§2). 새 포트는 모두 기본 구현이 있어 맥 어댑터를 고치지 않아도 빌드된다(CI 맥 clippy 통과).

---

## 1. 바뀐 공통 규칙·계약 (맥이 꼭 알아야 할 것)

1. **앱 이름·ID·경로가 바뀌었다**(브랜드 세션, 사용자 결정).
   - 화면(한글) **슉쇽**, 영문으로만 쓰는 곳 **ShookShok**, 파일·폴더·exe **Shookshok**, 앱 ID `com.ssamuniverse.shookshok`(소문자). 크레이트·npm·플러그인·이벤트 이름은 `screen-ink` 그대로.
   - 플러그인 상수 `APP_NAME`("슉쇽")·`APP_DIR`("Shookshok") — 화면 문구·폴더 이름은 여기서 읽는다.
   - 로그: tauri-plugin-log `LogDir`, 파일 이름 `Shookshok`. 맥 `~/Library/Logs/com.ssamuniverse.shookshok/Shookshok.log` **[확인됨 2026-11-26 맥 `tauri dev` — 첫 줄 `ShookShok ready`, 시각은 UTC]**, Windows `%LOCALAPPDATA%\com.ssamuniverse.shookshok\logs\Shookshok.log` [확인 필요]. 옛 `kr.screenink.desktop` 경로는 더 쓰지 않는다(맥에 옛 폴더가 남아 있지만 새 로그는 안 쌓인다).
   - 설정 폴더는 시작할 때 옛 `screen-ink` 폴더를 `Shookshok`으로 한 번 옮긴다. 포터블 표시 파일은 `Shookshok.portable`(옛 `screen-ink.portable`·`shookshok.portable`도 인정).
   - 맥 Dock·Launchpad 이름 슉쇽(`apps/screen-ink/src-tauri/Info.plist` `CFBundleDisplayName`).
2. **모드가 늘었다**: `Mode::{Idle, Capture, Draw, LiveZoom, Recording}`(`crates/ink-domain/src/mode.rs`). 다른 모드 키 = 지금 모드를 끝내고 전환(03 §3). 녹화 중 다른 모드를 끝내면 `rest_mode()`로 녹화 상태로 돌아간다.
3. **새 포트(모두 기본 구현 있음 — 맥은 아직 "지원 안 함")** — `crates/ink-app/src/ports.rs`
   - `OverlayHost::show_draw`(판서 오버레이), `flash_afterimage`(잔상), `request_commit`(주석 층 요청)
   - `Notifier::draw`·`draw_command`·`zoom`
   - `Magnifier`(+`wait_hidden`) — `InkService::set_magnifier()`로 꽂음, 기본 `NoMagnifier` → 맥은 `MacMagnifier`(M13, 2026-11-27)
   - `Recorder` — `set_recorder()`, 기본 없음 → **맥은 F3 녹화가 아직 없다**(M15)
   - `PackageRuntime`·`AutostartControl`·`Updater`(Windows MSIX용, 맥은 Noop)
   - `ElementCand.from_msaa`(`#[serde(default)]`, 사례 파일 하위 호환)
4. **InkClient(UI ↔ 앱) 추가** — `packages/ink-client/src/index.js`가 기준
   - 메서드: `drawReady`·`drawFinish`·`drawRestore`·`drawExport`·`drawIsland`·`drawPenMetrics`, `zoomInput`, `captureAnnotations`·`captureCommitAnnotated`
   - 이벤트: `screen-ink://draw`·`draw-command`·`zoom`·`afterimage`·`capture-commit`
   - 동작(`run`): `capture.toDraw`·`capture.copyHex`·`capture.saveLast`·`draw.acceptSuggestion`·`draw.undo`·`record.*`·`pins.closeAll`
5. **원본 바이트 IPC**: `draw_export`·`capture_commit_annotated`는 Tauri 2 `tauri::ipc::Request` 원본 본문으로 RGBA를 받는다(base64 없음). 주석은 본문 앞 `[머리 길이 u32 LE][머리 JSON][RGBA]`. **WKWebView(맥)에서도 원본 본문으로 온다 — 확인됨 2026-11-26**(`draw_export` Cmd+C → 클립보드 PNG 3840×2160, §2.1 결과). `capture_commit_annotated`는 M16에서.
6. **설정·단축키 변경**: `draw.shapeApply`(suggest 기본 | auto), `record.cursor`·`record.firstHintDone`·`capture.delay`·`capture.evidence`(숨김). 확대 키 `z-in`·`z-out` 칸 삭제 — 화면 안 키 43 → **41칸**(PRD v3.0.6, R-KEY-6).
7. **병렬 작업 규칙(다음에 또 나눌 때)**: 흐름 하나 = 워크트리 하나 = 브랜치 `stream/<이름>` = **전용 `CARGO_TARGET_DIR`**. 빌드 폴더를 워크트리끼리 같이 쓰면 같은 이름 산출물을 덮어써 틀린 결과가 난다(오늘 실제로 겪음). 메인 검사는 깨끗한 검사 전용 워크트리에서.
8. 공통 검사는 그대로(이전 문서 §1-4) + `npm run progress -- --check`. CI가 푸시마다 4작업을 돌린다.

---

## 2. 맥에서 할 일 (순서대로)

### 2.0 받기·빌드 확인
1. `git fetch origin && git checkout feat/foundation && git pull --rebase origin feat/foundation` → §8 해시 이후인지.
2. 공통 검사 4개 + `cargo clippy --workspace --exclude screen-ink --exclude ink-platform-windows --all-targets -- -D warnings`(CI 맥 작업과 같음).
3. `npm run app:dev` → 트레이·설정 창이 뜨는지, 이름이 슉쇽인지, 로그 경로(§1-1)를 확인해 고친다.

### 2.1 공통 기능을 맥 화면에서 확인 — M12·M16 (먼저, 반나절)
맥 키는 Alt = Option, Ctrl = Cmd(설정값 기준). F 키는 fn과 함께일 수 있다.
- **판서(M12)**: Option+1 → 정지 화면 위 판서. 펜·형광펜·지우개·글자 T(한글 IME 조합 중 키 무시)·고르기, 휠 확대(첫 휠 1.75×)·가장자리 밀기·1×에서 우클릭 = 끝, Esc 단계 끝내기, "판서를 보관했어요" → Option+Shift+1 복원(Retina 배율 맞춤), Cmd+C·Cmd+S = **늘 모니터 전체**, Option+3 칠판·W/K, 0.5초 멈춤 → "원으로 바꿀까요? · Enter". `show_draw`가 맥 오버레이에서 도는지(플러그인 `overlay.rs`), 배경이 번쩍이지 않는지.
- **원본 바이트 IPC**: Cmd+C 결과가 클립보드에 오는지 = `draw_export`가 WKWebView에서 원본 본문으로 오는지. 안 되면 [확인 필요]로 적고 대체 경로(설계: 03 §4) 결정.
- **캡처 주석·핀·칩·잔상(M16)**: F1 → 범위 확정 → 툴바 주석 도구 8종 → 복사·저장·[고정]에 주석이 들어가는지, 모자이크, 단색 범위의 HEX 칩, 확정 직후 0.4초 주황 잔상, 결과 섬 [판서로].
- 결과·수치(`mode enter draw [N ms]`)는 이 문서 §2.1 아래에 적는다.

**M12 결과(2026-11-26 저녁, 맥미니 4K@2 + FHD@1, 개발 빌드, 자동 입력 `bench/mac/input.swift`)**
- 진입 `mode enter draw`: 4K 9회 125~219 ms(p50 184 — capture 46~101 · show 5~16 · ui paint 46~111), FHD 125 ms, 칠판 64 ms. **목표 150 ms(G5)를 넘는다.** 큰 몫은 SCK 한 장(40~100 ms)과 웹뷰가 4K BMP를 읽어 첫 그림을 그리는 시간(46~111 ms). [할 일] 줄일 후보: 캡처처럼 네이티브 정지 창(`freeze`)을 먼저 띄우고 웹뷰 배경이 그려지면 걷기 · 미리 띄워 둔 SCK 흐름의 마지막 그림 쓰기.
- 펜 지연(소프트웨어 `pen latency`): p50 7~10 · p95 14~19 ms(목표 p95 ≤ 25 통과).
- 획 → Esc(보관) → Option+Shift+1: 세 획이 Retina 4K 제자리로 돌아옴(새 정지 화면 위). 끝난 뒤 섬 창은 몇 초 뒤 내려감(보관 안내).
- Cmd+C: 클립보드 PNG 3840×2160(FHD 모니터는 1920×1080), 획이 물리 px 제자리. Cmd+S: `~/Pictures/Shookshok/`에 같은 크기 PNG.
- 칠판 Option+3: 흰 바탕 · K = 검정 · W = 흰색. 도형 제안: 원을 그리고 0.9초 멈춤 → 점선 원 미리보기 → Enter = 반듯한 원. 1×에서 우클릭 = 끝.
- FHD 모니터에서 Option+1: 오버레이 1920,0 1920×1080 pt, 섬 가운데(x 2600 pt).
- 고친 것
  1. **판서를 끝낸 뒤 커서가 사라짐**(사용자 발견, Esc로 꺼도): 맥 WebKit은 `cursor: none`을 투명 커서로 바꾸는 것이라 창을 숨겨도 남는다 → 오버레이를 숨기기 전에 화살표 커서로(`ink_platform_macos::window::reset_cursor`, 플러그인 `hide`). 고친 뒤 Esc → 마우스를 움직이면 바로 화살표가 보임.
  2. **저장 파일 이름이 `screen-ink_…`**: 설정 파일이 값을 모두 적어 두어 이름을 바꾸기 전 기본 틀을 그대로 읽었다 → 읽을 때 옛 기본값은 새 기본값으로(`ink_domain::settings` `OLD_DEFAULTS`, 시험 1). 고친 뒤 `Shookshok_20261126_….png`. 학교 PC 설정 파일도 같았을 것.
  3. **휠 한 칸으로 확대가 안 됨**: 맥 WebKit 마우스 휠 한 칸 = 40 px < 문턱 50 → 두 칸째에 확대. 문턱 40으로(`inkEngine` `WHEEL_STEP_DELTA`·`captureAnnotate` `ANNOT_WHEEL_STEP`, 시험 2). 고친 뒤 한 칸 = 1.75×. Windows 한 칸(100 px)은 사건 하나에 한 단계라 그대로.
- 처음 두 번은 앞 획이 사라졌는데, 같은 순서·간격으로 다시 하면 재현되지 않았다. 그때 사용자가 컴퓨터를 쓰던 중이라 키 입력이 판서 창(키 창)으로 들어간 것으로 본다. **[확인 필요]** 손으로 쓰다가 또 보이면 기록.
- **[확인 필요 — 사용자 눈]** 섬·안내 문구는 캡처 제외 창이라 스크린숏에 안 찍힘(창 목록으로 떠 있는 것만 확인) · 진입 순간 배경 번쩍임 · 확대 중 가장자리 밀기. 글자 T 한글 IME는 M16에서 확인함(Option+1로 바로 들어간 판서는 **됨** — 2벌식 `a b c ␣` → "뮻 ").

**M16 결과(2026-11-26 저녁, 같은 환경)** — 섬 내용은 접근성(AX)으로 읽었다(캡처 제외 창이라 스크린숏 불가).
- 캡처 진입 `overlay ready` 64~103 ms. 범위 끌기 → 오른쪽 아래 밖에 툴바, 화면 구석 범위는 위쪽 화면 안으로.
- 주석: 펜·사각형·모자이크 → 복사 PNG 800×500에 셋 다 들어감 = **`capture_commit_annotated` 원본 바이트도 WKWebView에서 온다(확인됨)**. 글자 도구 숫자·영문(Playwright WebKit 미리보기) 됨.
- 잔상: 확정 뒤 주 모니터 오버레이가 0.2~0.4초 남았다가 내려감(창 목록 0.1초 간격).
- 결과 섬: "복사했어요 · 800×500 · 한글·PPT에 ⌘V" · [저장] · [판서로] · 단색 범위 HEX 칩 "#F1F0F0 복사" → 누르면 클립보드 `#F1F0F0`. [판서로] = `mode enter draw [71 ms] (from capture, scale 3.8, frame reused …)`(목표 150 통과).
- 핀(R-CAP-29): 두 번 클릭 = 닫기, 휠 한 칸 = 125%(커서 자리 고정).
- 고친 것
  1. **결과 섬이 0.2초 만에 사라짐**: 확정은 모드 끝(End) 바로 뒤 결과(Result)를 보내는데, End로 섬이 빈 순간 화면이 보낸 `island.hidden`이 늦게 닿아 방금 띄운 결과 섬 창을 숨겼다 → 화면이 120 ms 기다렸다가 그때도 비었을 때만 숨김(`apps/screen-ink/src/App.svelte`). 고친 뒤 결과 섬 약 2초(설계값 — 동작 줄이기 설정이면 2초) 뒤 화면이 스스로 걷음. Windows에도 같은 경합이 있었을 것.
  2. **핀이 혼합 배율에서 어긋남**: Tauri 물리 좌표로 놓아 4K에서 50 pt 아래, FHD에서 (−190, −100) pt → 맥은 포인트로 직접(`window::place_rect`·`frame_phys`, 변환 = `Space::rect_to_points` 하나 + 시험). 고친 뒤 FHD 범위 2300,300 400×250 = 핀 2300,300 400×250.
  3. **맥 결과 섬 안내가 "Ctrl+V"**: 범위·창 캡처·판서 복사 안내 → 맥 `⌘V`(`paste_text`), [저장] 키는 설정 `c-save` 값(`key_text`).
- **[할 일 — 버그, 원인 못 찾음] 캡처 중 글 상자에서 한글 조합이 안 될 때가 있음**(맥, 2026-11-27 새벽 조사): 캡처 주석 글자 도구(그리고 캡처 뒤 [판서로] 판서 글자)에서 2벌식 `a b c ␣`가 안 들어가고 숫자만 들어간다. **한글 키는 웹 페이지까지 아예 오지 않는다**(문서 전체 keydown·beforeinput·composition 0건, 숫자는 keydown·insertText 옴) → 페이지보다 앞, 맥 입력기 ↔ WKWebView 사이 문제. 같은 실행에서 Option+1로 연 판서 글자는 늘 된다(두 번째 상자도 됨).
  - **실행마다 다르다**: 같은 코드로 된 실행(23:15 첫 실행, 실험 실행 두 번)과 안 된 실행이 섞였다. 한 실행 안에서는 늘 같다(3/3).
  - 배제(그것만 꺼도 계속 실패): 네이티브 정지 패널 · 두 번째 모니터 오버레이 · 섬(앞으로 올리기·자리 잡기·Tauri show) · 범위 끌기(클릭으로 골라도 실패) · 글 상자 초점 다시 잡기 · 우리 앱 활성화 · 보안 입력 · 문서별 입력 소스(입력 소스는 2벌식 그대로) · 캡처 화면을 세션마다 새로 붙이기(한 번 됐다가 다음 실행에서 실패).
  - 다음에 볼 것: 그 실행의 맥 입력기 상태(실패 실행에서 다른 앱에서는 한글이 되는지), `NSTextInputContext.current`가 오버레이 웹뷰의 것인지(Rust에서 로그 — `objc2`로 `currentInputContext`·`client` 찍기), 우회안 = 캡처 주석 글자만 네이티브 입력 칸(NSTextField 패널)으로 받기.
  - 조사 도구: 화면 오류·`console.error`가 이제 앱 로그로 온다(`ui <창>: …`, `ui_log` 명령). 키·조합 기록이 필요하면 글 상자 `oninput`·`compositionstart`에 `console.error`를 잠깐 넣으면 로그에서 보인다.
- **[완료 2026-11-27] 판서 중 F1 → 커서 모니터 캡처 화면이 빈 화면**(입력 안 받음, Esc로도 안 닫힘): 클라이언트 `subscribe`가 부를 때마다 `listen`을 새로 걸어(IPC 왕복), 판서 → 캡처 전환에서 새로 붙은 캡처 화면이 그 사이 온 세션 사건(`overlay`)을 놓쳤다 → `packages/ink-client/src/hub.js`(창마다 듣기 한 번 + 마지막 캡처 세션을 늦게 붙은 구독자에게 다시 줌, 캡처가 아닌 상태가 오면 지움) + 시험 4. Windows도 같은 화면 코드라 같은 버그였을 것.

**M9 결과(2026-11-26 밤)** — `crates/ink-platform-macos/src/ax.rs`(`AxIndex`, Windows `uia.rs`와 같은 구조), `Control::from_ax`(+ 시험), 플러그인 `element_source` 맥 분기.
- 요소마다 역할·부역할·자리·크기·자식을 `AXUIElementCopyMultipleAttributeValues` 한 번으로, 요청 제한 400 ms, 창 하나 예산 300 ms, 방문 4000·저장 1500·깊이 12. 창은 CG 경계와 AX 창 자리·크기(±2 pt, 없으면 겹침 최대)로 찾는다. 사각형은 `Space::rect_to_phys` 하나로 바꾸고 창 안으로 자른다. 글자 값·제목은 읽지 않는다.
- 실측(이 셸 = 손쉬운 사용 있음, `cargo test -p ink-platform-macos ax::tests::실제_맨앞_창_색인 -- --ignored --nocapture`): 맨 앞 앱 창 요소 48개 42 ms(Group 30·Button 14·Tab·ToolBar·Text).
- 개발 앱(`npm run app:dev`)은 손쉬운 사용 권한이 **터미널 앱** 몫이라 지금은 `ax index skipped: accessibility not granted` → A 근거 없이 W·V로 캡처(정상 대체). **[확인 남음]** 권한을 준 실행(서명한 `.app` 또는 터미널에 손쉬운 사용)에서 `capture.evidence: true`로 초록 `A` 마디, `ax index N elements in M ms`, 느린 앱 강등(500 ms×2).

**M10 결과(2026-11-26 밤)** — 권한 스마트 흐름(사용자 결정: 설정 앱을 찾아다니지 않게)
- 공통: `SystemProbe::permissions()`(기본 None = Windows), `InkState.permissions {screen, accessibility, screenRevoked, accessibilityRevoked}`, 숨은 설정 `mac.hadScreenPerm`·`mac.hadAxPerm`(있으면 기록 → 전에 있었는데 없으면 풀림), 섬 사건 `durationMs`(권한 섬 15초·안내 120초).
- 화면 기록 없음(캡처·판서·바로 캡처): 문제 섬 "화면 기록 권한이 필요해요" + **[화면 기록 켜기]**(`perm.screen`) → 맥 `CGRequestScreenCaptureAccess` + 설정 칸(`x-apple.systempreferences:…?Privacy_ScreenCapture`) 열기 + 안내 섬 "화면 기록에서 슉쇽을 켜 주세요 · 켰으면 다시 시작"(**[다시 시작]** = `app.relaunch`). 화면 기록은 이 프로세스에서는 다시 시작해야 반영된다(macOS도 "종료 후 다시 열기"를 묻는다).
- 손쉬운 사용 없음: 이번 실행 첫 캡처의 모드 섬에 칩 **"요소까지 잡기 · 손쉬운 사용 켜기"**(`perm.accessibility`) 한 번 → 캡처를 끝내고 `AXIsProcessTrustedWithOptions(prompt)` + 설정 칸 열기 → 1초마다 확인, 켜지면 "손쉬운 사용을 켰어요 · 이제 요소까지 잡아요"(다시 시작 없음, 2분까지).
- 시작 2초 뒤: 화면 기록이 풀렸으면 "업데이트 뒤 화면 기록 권한을 다시 켜야 해요" + [화면 기록 켜기].
- 첫 실행 시트(맥, 화면 기록 없음): 경고 줄 + [화면 기록 켜기] → 누른 뒤 [켰으면 다시 시작]. 미리보기 `?screen=first&platform=macos&perm=screen-off|screen-revoked`.
- 시험: `crates/ink-platform-fake/tests/permissions.rs` 5개(권한 개념 없음 · 풀림 감지 · 처음부터 없음은 풀림 아님 · 칩 한 번 · 켜면 칩 없음), 캡처 시험의 권한 없음 단추.
- **[확인 남음]** 실제 단추 흐름(사용자 화면에 시스템 설정·권한 창이 떠서 자동 시험하지 않음): 서명한 `.app`에서 `tccutil reset ScreenCapture com.ssamuniverse.shookshok` 뒤 첫 실행 → 단추 → 켜기 → 다시 시작 / 손쉬운 사용 칩 → 켜는 순간 안내.

**M-sign 준비(2026-11-26 밤)** — `scripts/mac-build-dmg.sh`(신원 확인 → `tauri build --bundles app,dmg` → `codesign --verify` · DR을 `target/mac-dr.txt`에 남기고 지난번과 비교), 안내 `docs/handoff/mac-self-sign.md`(만드는 사람: 키체인에서 자체 서명 코드 서명 인증서 한 번 / 받는 사람: "그래도 열기"). **막힘: 이 맥 키체인에 코드 서명 신원이 없다**(`security find-identity -v -p codesigning` = 0) — 인증서 만들기는 사람이 한다(§4).

**M14 결과(2026-11-26 밤)** — `crates/ink-platform-macos/src/window_shot.rs`, `MacScreen::prepare_window`·`capture_window`(포트 그대로).
- F2 고르기 중 창이 정해지면 뒤에서 SCK 창 목록을 받아 그 창을 쥐어 둔다 → 확정 때 `SCContentFilter(desktopIndependentWindow:)` + `ignoreShadowsSingleWindow` + sRGB BGRA(곧은 알파), 크기 = 서비스가 넘긴 창 사각형. 300 ms 안에 안 오거나 RGB 전부 0이면 오류 → 서비스가 정지 스냅숏 자르기로 대체(`windowshot::after_primary`, 맥은 표시 선호도 없음 = 0).
- 실측(이 셸, `cargo test -p ink-platform-macos window_shot::tests -- --ignored --nocapture`): FHD 창 1920×1050 **48 ms**, 4K 창 3840×2100 **114~161 ms**(준비 뒤). 그림은 저장하지 않는다.
- **[확인 필요]** 둥근 창 귀 투명: Chromium 계열 창(Claude·Aside, 화면을 채운 창)은 SCK가 네모 그대로 줬다(모서리 알파 255). 네이티브 창(Finder·메모)에서 귀가 투명으로 오는지, 아니면 Windows처럼 반지름으로 깎아야 하는지(macOS 판마다 반지름이 다름) 확인. 실제 앱 F2 흐름(가려진 창·보호 화면 안내)도 확인 남음.

**M15 결과(2026-11-26 밤)** — `crates/ink-platform-macos/src/recorder.rs`(`MacRecorder`, 포트 그대로), 플러그인 `set_recorder` 맥 분기, 새 의존성 `objc2-av-foundation`·`objc2-core-media`·`dispatch2`(+ SCK 기능 `dispatch2`·`objc2-core-media`).
- `SCStream`(모니터 = 디스플레이 필터, 범위 = 같은 필터 + `sourceRect`, 창 추적 = `desktopIndependentWindow`) → 표본 출력 객체(`define_class!` `ShookshokRecOutput`, 직렬 큐) → 완전한 프레임(`SCFrameStatus::Complete`)만 `AVAssetWriterInput`(H.264 High, 서비스가 준 비트레이트·fps, 키프레임 2초) → mp4. 픽셀 '420v', 커서는 설정대로.
- 마무리: 스트림 정지 완료 → `markAsFinished` → `finishWriting`(moov). 프레임 0개·실패면 오류 + 임시 파일 남김(복구). 창 추적 중 창이 닫히면 `stream:didStopWithError:` → `REC_WINDOW_CLOSED`.
- 파일 처리(남은 녹화 `rec-*.mp4` 찾기·저장 폴더로 옮기기·같은 이름 ` (2)`)는 Windows와 같은 규칙, 남은 공간은 `statvfs`. 마이크는 아직 없음(`mic_supported` false).
- 우리 섬·빨간 꺾쇠·정지 패널은 창 공유 끔이라 영상에 안 찍히고, 판서·확대 오버레이는 찍힌다. 빨간 꺾쇠 창도 맥에서는 포인트로 그 모니터를 덮게 고침(혼합 배율 어긋남 방지).
- 실측(이 셸, `cargo test -p ink-platform-macos recorder::tests -- --include-ignored --nocapture`): 4K 주 모니터 2초 = 3840×2160 **프레임 47개 · 361 KB · 상자 ftyp/mdat/moov**(재생 가능). 영상은 바로 지웠다.
- **[확인 남음]** 실제 앱 F3(범위·모니터·창 추적), 10분 녹화, 0.3초 즉시 중지 재생, 영상에 섬·꺾쇠 0 px, 녹화 중 판서가 영상에 남는지, 잠금·모니터 분리 자동 중지와 복구 섬.

**M13 결과(2026-11-27 맥) — [확인 남음: 사용자 눈·4K CPU·서명 앱]**
- 돋보기(`magnifier.rs`): 그 모니터를 덮는 NSPanel(클릭 통과·창 공유 끔·모든 Space·층 999) + CALayer. `SCStream`(그 디스플레이, `excludingApplications:[우리 앱]` = R-ZOOM-4, 60 fps, BGRA, 커서 없이)의 완전한 프레임 IOSurface를 메인 대기열에서 `layer.contents`로(복사 없음, CVPixelBuffer를 쥐어 SCK 풀이 붙은 그림을 다시 쓰지 않게), 원본은 `contentsRect`. **크기는 1/배율(고른 배율)** — 정수로 올린 원본 크기로 늘이면 가로·세로 배율이 조금 달라져 커서와 그림이 어긋난다(Windows `MagSetWindowTransform`과 같은 모델). `start`는 첫 프레임(≤ 800 ms)을 채널로 받아 붙인 뒤 앞으로.
- 커서: SCK는 **숨긴 커서를 찍지 않는다**(실측) → 스트림은 커서 없이, 시스템 커서 그림(`currentSystemCursor`, 10 Hz로 모양 갱신)을 층 하나로 확대 자리에 그린다(최대 3배 크기). 진짜 커서는 비공개 `SetsCursorInBackground`(dlsym, 없으면 물러남) 뒤 `CGDisplayHideCursor` — 그 모니터 안에서만 숨기고 밖이면 보인다, 끝·`Drop`에서 되돌린다. 실기: 확대 커서 끝 = 진짜 커서 자리(±2 px), Esc 뒤 커서 보임.
- 휠·Esc(`wheel.rs`): 전용 스레드 `CGEventTap`(scrollWheel + keyDown, 키를 못 받으면 스크롤만). 확대 모니터 안·Shift 없음만 먹는다. **위로 굴림 = 더 확대**(처음 짠 것은 거꾸로였다 — 서비스 규약 `dy < 0` = 확대). 마우스 휠 = 칸(`Wheel{dy}`), 트랙패드 = 40 pt가 한 칸인 소수 칸(`Scroll{notches}`, 새 입력). 보조 키 없는 Esc = `Exit`(전역 Esc 단축키가 못 잡힐 때의 대비).
- 확인용 환경 변수 `SCREEN_INK_ZOOM_SHARE=1`: 돋보기 창 공유를 켜 `screencapture`에 보이게(배율·뒤집기 실측에만).
- **확대 고도화(공통 `ink-domain::zoom`·`ink-app::live_zoom`, Windows에도 같이 적용)**
  - 배율: 단계표(1.25·1.5·2·2.5·3·4·6·8) → **1×~8× 연속**. 휠 한 칸 = ×2^(1/8)(≈ 9%, 두 배에 8칸), 휠은 격자 위에 선다(×1.0·×2.0·×4.0 정확히), 트랙패드는 격자 없이. 1×에서도 끝나지 않는다(끝 = Esc·Alt+2). 섬·HUD 글자는 소수 한 자리(`×2.2` — 격자 칸마다 다르게 읽힘). `wheel_target`.
  - 부드럽게: 휠은 **목표 배율**(`LiveZoomView.target`)만 옮기고, 틱마다 로그 공간 지수 감쇠(`SCALE_ALPHA` 0.3/프레임, 프레임 독립 — 90%까지 ≈ 110 ms)로 다가간다. 전환 중에도 커서 아래 점 고정. `step_scale`.
  - 마우스 위치대로(`zoom.follow = pointer`, 새 기본): 원본 왼쪽 = 모니터 왼쪽 + (커서 + ½ − 모니터 왼쪽) × (1 − 1/배율) → **커서의 확대 화면 자리 = 실제 커서 자리**(반 화소 × 배율 안). 가리킨 쪽이 확대되고(모서리 → 모서리, 가운데 → 가운데), 클릭 자리가 곧 보이는 자리. 옛 방식은 `smooth`(가운데로 — 0.35 + 10% 죽은 영역)·`instant`로 남김. 저장된 옛 기본값 `smooth`는 새 기본값으로 바뀐다(`OLD_DEFAULTS`).
  - 시험: 도메인 `휠_한_칸은_두_배의_8분의_1이고…`·`배율_전환은_목표로_부드럽게…`·`마우스_위치대로는_커서가…`, 통합 `live_zoom.rs`(첫 진입 기대값을 비례 지도로, 휠 시험을 목표·틱 방식으로, 가운데 따라가기 시험은 `smooth`를 켜고), 가짜 클라이언트 `zoom.test.js`.
- **[확인 남음]** 사용자 눈(확대 전환 느낌, 트랙패드 속도 40 pt/칸이 알맞은지), Windows 실기(같은 공통 로직 — 돋보기 어댑터는 그대로), 4K 60 fps CPU·발열, 확대 중 Option+1 이어받기 화면(자동 시험은 통과), 서명한 `.app`(권한 주체가 앱이 되면 비공개 커서 속성·탭이 그대로 되는지).

### 2.2 맥 트랙 원래 순서 — M9·M10·M17·M-sign
- **M9 AX 요소 색인**(P0, 1.5일): 시안 [I-mac.md](../plans/2026-11-26/I-mac.md) §2.1. Windows UIA 일꾼(`ink-platform-windows/src/uia.rs`)과 MSAA 대체(`msaa.rs`)의 구조·시간 예산을 본보기로.
- **M10 권한 스마트 흐름**(1일): I-mac.md §2.2. 권한 허용은 사용자에게 부탁(직접 바꾸지 않음).
- **M17 첫 실행 시트 F키 한 줄**(사용자 결정 #9): "fn+F1 또는 키보드 설정의 '표준 기능 키'" — 04 말투로 짧게. 브랜드 세션이 고친 `FirstRunSheet.svelte`에 덧붙이기.
- **M-sign**: 자체 서명 dmg·업데이트 뒤 권한 유지(I-mac.md §2.3).
- 확인만 남은 M3~M6·M8·M11은 이전 문서 §2.1 표대로.

### 2.3 맥에서 새로 만들 어댑터 — M14·M13·M15 (큰 일)
Windows 구현을 본보기로 같은 포트에 맥 구현을 꽂는다. 공통 계약은 바꾸지 않는다.
- **M14 F2 창 캡처**(0.5일): Windows `wgc_window.rs`의 흐름(미리 준비 → 확정 → 첫 프레임 시간 제한 → 대체 경로 → 보호 화면 = RGB 정확히 0 판정 `ink-domain`)을 ScreenCaptureKit 창 필터로. 둥근 모서리 귀만 투명(사용자 결정, R-OUT-3과 일치).
- **M13 실시간 확대**(1.5일): PRD R-ZOOM-1 맥 = **SCStream → Metal**. 포트 `Magnifier`, Windows `magnifier.rs`(전용 스레드·미리 만들기·우리 창 제외)·`live_zoom.rs`. 사용자 결정: 확대 중 전역 키는 Esc만, 휠 = 배율(2026-11-27부터 1×~8× 연속, 1×에서 멈춤), Shift+휠은 아래 앱으로. **[완료 2026-11-27 — §2.1 M13 결과]**
- **M15 F3 녹화**(1.5일): 포트 `Recorder`, Windows `recorder.rs`(MF 직접, 하드웨어 H.264, 1080p30 **5 Mbps** — 사용자 결정). 맥 = SCStream + AVAssetWriter(H.264). 창 클릭 = 창 추적(사용자 결정 #6). 자동 중지·임시 파일 복구는 공통(`record.rs`).

### 2.4 맥에서 해도 되는 플랫폼 무관 일
- 브라우저 미리보기 확인: `npm run ui:dev` → `?draw=1`(옵션 `&board=1`·`&apply=auto`), `?screen=island`, `?screen=settings&tab=draw`.
- **[완료 2026-11-26 맥] 결과 섬이 떠 있는 동안 Alt+1 = [판서로]**(메인 결정 — 키캡 `Alt 1`과 맞춤): 확정 뒤 `RESULT_ISLAND_TO_DRAW`(2초 — UI 결과 섬 1.2초, 동작 줄이기면 2초) 안의 판서 키 = `capture_to_draw_at`(재캡처 0), 지나면 보통 판서. 시험 `handoff.rs` `결과_섬이_떠_있을_때_alt1은_판서로와_같고_지나면_보통_판서`.
- **[완료 2026-11-26 맥] 판서 중 트레이로 실시간 확대를 켜면 판서를 끝내지 않는 버그**: `start_zoom`이 판서를 끝내고(보관은 늦은 끝으로) 확대 — Alt+2와 같다. 시험 `draw.rs` `판서와_실시간_확대는_서로_끝내고_전환`에 덧붙임.
- **[할 일] 판서 엔진 확장**(캡처 주석 일꾼 요청, 지금은 바깥에서 우회): 고정 도형 도구, 모자이크 도구, `setWidth(px)`, `drawMosaic` 박스 평균(미리보기 = Rust 결과 화소 일치), `DrawCanvas` 투명 바탕·보기 원점 옵션.
- **[할 일] 여러 모니터에 걸친 주석 동기화**(`captureAnnotSync`) — 지금은 툴바 모니터 창에서만 그림.
- 단계 7.7 디자인 관문, 문서 정리.

---

## 3. Windows에서 할 일 (학교 PC로 돌아갔을 때)

### 3.1 화면 확인 묶음 — 사용자에게 미리 묻고 한 번에(합계 약 3시간)
1. **인식**(1.7~2시간): `bench/recog/RUNBOOK-windows.md` 순서대로 — RB-15·16 실제 창(`cargo test -p ink-platform-windows --test rb15_16 real:: -- --ignored --test-threads=1`), DPI·MSAA 실측(125%/150% PC 필요 — 배율 변경은 사용자 승인), RB-20 멈춘 앱, 사례 17종(RB-01~14·17·18·19로 해석)·기준선, G3, Snipaste 대조, 붙여 넣기.
2. **판서 G5**: 펜 지연 로그 `pen latency n=.. p50=.. p95=..`(소프트웨어 p95 ≤ 25 ms), 240 fps 촬영 p95 ≤ 50 ms, 진입 `mode enter draw` p95 ≤ 150 ms. 휠 문턱을 50 → 40 px로 낮췄다(맥 M12) — Windows 마우스 휠 한 칸 = 한 단계 그대로인지, 터치패드가 너무 민감하지 않은지. 저장 파일 이름이 `Shookshok_…`로 나오는지(옛 설정 파일의 `screen-ink_…` 틀을 읽을 때 바꿈).
3. **실시간 확대 M6**: `bench/m6-zoom.md`(1080p ≥ 30 fps, 4K는 4K 모니터·DSR에서). 시스템 커서가 하나만 보이는지(`magnifier.rs` `HIDE_SYSTEM_CURSOR`), Shift+휠 통과, 끝낸 뒤 방향키·Esc 정상.
4. **F2·F3·바로 캡처**: 가려진 창 F2(노란 테두리 없는지 육안), 둥근 창 귀 투명, 보호 화면 안내, 바로 캡처 4종(Shift+F1·Ctrl+F1·Shift+F2·Alt+F1 지연 — 카운트다운 중 Esc 취소), F3 1080p30 10분·0.3초 즉시 중지 재생, 영상에 섬·꺾쇠 0 px, Win+L·모니터 분리·강제 종료 뒤 복구 섬, `recorder warmed in`·`encoder ready in`.
5. **주석·잔상·이어받기**: 툴바 14단추 넘침, 도구별 결과 위치·색(150%), 글자 도구 한글, 첫 Esc = 도구 해제, 잔상 중 클릭 통과, Alt+2 → Alt+1(배경에 돋보기 안 찍힘, ≤150 ms), [판서로](`frame reused`, ≤150 ms).
6. **MSIX**(사용자 승인 뒤): 자체 서명 설치, StartupTask, `RequestAccessAsync(Borderless)` 동의 창 문구.

### 3.2 Windows 남은 구현
- **D10** 업데이트 어댑터(StoreContext·PackageManager — 스파이크 §5) + 새 버전 팝업 UI(`UpdateSheet.svelte`, 11 §7.2).
- **D8·D9** 서명 직접 배포판·`.appinstaller` 호스팅(SignPath 승인 뒤).
- MSIX 산출물 이름을 브랜드 규칙대로 `Shookshok_…msix`·`Shookshok.appinstaller`로(지금 소문자 `shookshok_…`) — 매니페스트·스크립트·`APPINSTALLER_URI` 함께.
- 녹화 마이크(R-REC-3 P1, WASAPI + AAC — 지금은 "준비 중" 안내), 패키지판 `RequestAccessAsync(Borderless)` 호출.
- MSAA 마감 초과를 느린 앱 강등에 세기, 돋보기 기다림을 `wait_hidden`으로 옮기기(Esc 끝내기 16 ms 줄이기).
- 6.5 창 항상 위 고정(P1 — 사용자 결정: 필수 기능 뒤), 단계 7 안정화(7.1~7.6), 확인만 남은 J 묶음(0.4~2.6).

---

## 4. 사용자(선생님)가 직접 할 것
- **D1** Microsoft Partner Center 신분증 인증·앱 이름 예약 → GitHub 변수 `STORE_PACKAGE_NAME`·`STORE_PUBLISHER`.
- **D11** SignPath Foundation 신청(GitHub 2FA 필요, README 끝 `## Code Signing Policy` 절은 넣어 둠) → `SIGNPATH_*` 변수·`SIGNPATH_API_TOKEN` 비밀값.
- 화면 확인 시간(§3.1, 약 3시간)과 DPI 실측 방법(125%/150% PC 또는 배율 변경 허락).
- 캡처 주석 아이콘 8종(지금 임시 글리프) — 브랜드에서 확정.
- **맥 자체 서명 인증서 한 번 만들기**(M-sign) — `docs/handoff/mac-self-sign.md` 1절(키체인 접근 › 인증서 지원 › 인증서 생성, 자체 서명 루트·코드 서명). 만든 뒤 `APPLE_SIGNING_IDENTITY="…" sh scripts/mac-build-dmg.sh`.
- 맥 권한 흐름 눈 확인(M10 §2.1 결과) — 서명한 `.app`에서 단추 흐름. 개발 실행(`npm run app:dev`)은 권한이 터미널 앱 몫이라 손쉬운 사용(A 근거)이 없다.

---

## 5. 결정 기록

- 사용자 결정 11건 + 메인 기술 판단: [work-streams.md §6](../plans/work-streams.md). 요약 — 도형 보정 제안형 기본(설정으로 바로 바꾸기), 확대 중 복사·저장 = 모니터 전체, 창 녹화 = 창 추적, 앱 이름 규칙, 6.5는 뒤로, 모자이크 10 px + Ctrl+휠, 결과 섬 키캡 생략, 확대 중 휠 = 배율·Shift+휠 통과, 맥 F키 안내 첫 실행 한 줄, 확대 중 전역 키 Esc만, 1.25배 멈춤, 둥근 창 귀 투명, 녹화 5 Mbps.
- 오늘 정한 기술 판단: 녹화는 windows-capture 대신 **Media Foundation 직접**(MSRV·지연 실측), 보호 화면 = **RGB 정확히 0**, 보관 판서 복원은 **실행 취소 가능한 명령 하나**, 판서 Space = 1× 복귀만, 캡처 주석 첫 Esc = 도구 해제, 지연 캡처 취소 = 카운트다운 중 임시 Esc, 결과 섬이 떠 있을 때 Alt+1 = [판서로](할 일), 판서 중 Alt+2 = 판서 끝내고 확대.
- PRD: v3.0.6(확대 키 Esc만·41칸·R-ZOOM-2 휠 규칙). R-OUT-3("투명 픽셀이 없으면 알파 제거")은 귀 투명 결정과 이미 맞는다.

---

## 6. 브랜치·작업 폴더

- 흐름 브랜치 `stream/{cap-chip, ink-engine, draw-mode, win-recog, live-zoom, msix, window-record, cap-annotate, handoff, draw-ui}`는 **모두 feat/foundation에 병합했다**(원격에 따로 올리지 않음).
- Windows PC의 `D:\dev-cache\screen-ink\wt\*` 워크트리와 `target-*` 빌드 폴더는 지워도 된다(`git worktree remove`). 스파이크 시험 코드는 `D:\dev-cache\screen-ink\spikes\`(보고서는 저장소에 있음).

## 7. 맥 첫 세션

[mac-first-session-prompt.md](mac-first-session-prompt.md)의 블록을 첫 메시지로 붙여 넣는다.

## 8. 기준 커밋

- 코드 기준: **`ae7766e`** — 흐름 11개를 모두 병합한 마지막 병합(window-record). 그 위에 진척도·이 문서 마감 커밋이 있다(`git log -1 -- docs/handoff/2026-11-26-mac-handoff.md`).
- 마감 시점 진척: 전체 62.8% · 1차 계획 75.5% · 맥 트랙 41.7% · Windows 배포 42.4%([README](../progress/README.md)).
- 마감 검사(깨끗한 검사 전용 워크트리, CI와 같은 범위): Rust 시험·Windows 순수 판정 시험·clippy -D warnings·npm test·npm run check·progress --check. 결과는 마감 커밋 메시지에 적는다. 실제 화면 확인은 하지 않았다(§3.1).
