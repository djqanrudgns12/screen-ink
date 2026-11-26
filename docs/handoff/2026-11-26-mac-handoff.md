# 인수인계 — 2026-11-26 마감 (Windows 병렬 작업 → 맥에서 이어서)

- 기준: 브랜치 `feat/foundation`, 이 문서를 넣은 커밋(맨 아래 §8에 해시). 진척 = [docs/progress/README.md](../progress/README.md).
- **이 문서가 맥 작업 지시서다.** 이전 문서 [2026-11-25-mac-windows.md](2026-11-25-mac-windows.md)의 §1 공통 규칙·§5 파일 지도는 그대로 유효하고, 바뀐 것은 아래 §1에 적었다.
- 오늘 Windows에서는 메인 세션이 일을 흐름 11개로 쪼개 Claude 일꾼·제미나이(antigravity) 네온에게 동시에 맡기고 병합했다. 운영 기록 = [docs/plans/work-streams.md](../plans/work-streams.md), 설계 시안 = [docs/plans/2026-11-26/](../plans/2026-11-26/index.md), 실측 = [spikes-windows.md](../plans/2026-11-26/spikes-windows.md).
- 표기: **[완료]** 코드·자동 시험 끝 · **[확인 남음]** 실제 화면 확인만 남음 · **[할 일]** · **[확인 필요]** 아직 사실 확인 안 한 가정

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
| F2 창 캡처(WGC)·바로 캡처 4종·F3 녹화(MF)·자동 중지·복구·빨간 꺾쇠 (6.1~6.5, Windows) | [확인 남음] · 마이크는 자리만 | `ink-platform-windows/src/{gpu,wgc_window,recorder}.rs`, `ink-app/src/record.rs`, `ink-domain/src/recording.rs`, 플러그인 `rec_frame.rs`, `RecordFrame.svelte` | 테두리 육안·10분 녹화 |
| 인식 마무리 (3b-dpi·3b-msaa·RB-15/16·시험 앱·Snipaste 판독) | [확인 남음] | `ink-platform-windows/src/{dpi,msaa}.rs`, `tests/rb15_16.rs`, `bench/recog/`(RUNBOOK-windows.md) | 화면 절차 1.7~2시간 |
| MSIX 포장 D2·패키지 모드 D3 | [완료]·실설치 확인 남음 | `scripts/package-msix.ps1`, `apps/screen-ink/src-tauri/packaging/msix/`, `ink-domain/src/package.rs`, `ink-platform-windows/src/package.rs`, `.github/workflows/release-msix.yml` | 자체 서명 설치(사용자 승인) |
| CI 상시 실행 | [완료] | `.github/workflows/ci.yml`(푸시마다 ubuntu·Windows·맥·UI, Windows 순수 판정 시험 추가) | — |
| 앱 이름·아이콘 (사용자 브랜드 세션) | [완료] | 커밋 8796250·848d271, `docs/brand/README.md` | — |

**맥은 오늘 코드가 바뀌지 않았다.** 다만 공통 코드가 크게 늘어 맥에서 확인할 것이 생겼다(§2). 새 포트는 모두 기본 구현이 있어 맥 어댑터를 고치지 않아도 빌드된다(CI 맥 clippy 통과).

---

## 1. 바뀐 공통 규칙·계약 (맥이 꼭 알아야 할 것)

1. **앱 이름·ID·경로가 바뀌었다**(브랜드 세션, 사용자 결정).
   - 화면(한글) **슉쇽**, 영문으로만 쓰는 곳 **ShookShok**, 파일·폴더·exe **Shookshok**, 앱 ID `com.ssamuniverse.shookshok`(소문자). 크레이트·npm·플러그인·이벤트 이름은 `screen-ink` 그대로.
   - 플러그인 상수 `APP_NAME`("슉쇽")·`APP_DIR`("Shookshok") — 화면 문구·폴더 이름은 여기서 읽는다.
   - 로그: tauri-plugin-log `LogDir`, 파일 이름 `Shookshok`. 맥 `~/Library/Logs/com.ssamuniverse.shookshok/Shookshok.log`, Windows `%LOCALAPPDATA%\com.ssamuniverse.shookshok\logs\Shookshok.log` **[확인 필요 — 맥에서 첫 실행 때 실제 경로를 확인해 이 줄을 고칠 것]**. 옛 `kr.screenink.desktop` 경로는 더 쓰지 않는다.
   - 설정 폴더는 시작할 때 옛 `screen-ink` 폴더를 `Shookshok`으로 한 번 옮긴다. 포터블 표시 파일은 `Shookshok.portable`(옛 `screen-ink.portable`·`shookshok.portable`도 인정).
   - 맥 Dock·Launchpad 이름 슉쇽(`apps/screen-ink/src-tauri/Info.plist` `CFBundleDisplayName`).
2. **모드가 늘었다**: `Mode::{Idle, Capture, Draw, LiveZoom, Recording}`(`crates/ink-domain/src/mode.rs`). 다른 모드 키 = 지금 모드를 끝내고 전환(03 §3). 녹화 중 다른 모드를 끝내면 `rest_mode()`로 녹화 상태로 돌아간다.
3. **새 포트(모두 기본 구현 있음 — 맥은 아직 "지원 안 함")** — `crates/ink-app/src/ports.rs`
   - `OverlayHost::show_draw`(판서 오버레이), `flash_afterimage`(잔상), `request_commit`(주석 층 요청)
   - `Notifier::draw`·`draw_command`·`zoom`
   - `Magnifier`(+`wait_hidden`) — `InkService::set_magnifier()`로 꽂음, 기본 `NoMagnifier` → **맥은 실시간 확대가 아직 없다**(M13)
   - `Recorder` — `set_recorder()`, 기본 없음 → **맥은 F3 녹화가 아직 없다**(M15)
   - `PackageRuntime`·`AutostartControl`·`Updater`(Windows MSIX용, 맥은 Noop)
   - `ElementCand.from_msaa`(`#[serde(default)]`, 사례 파일 하위 호환)
4. **InkClient(UI ↔ 앱) 추가** — `packages/ink-client/src/index.js`가 기준
   - 메서드: `drawReady`·`drawFinish`·`drawRestore`·`drawExport`·`drawIsland`·`drawPenMetrics`, `zoomInput`, `captureAnnotations`·`captureCommitAnnotated`
   - 이벤트: `screen-ink://draw`·`draw-command`·`zoom`·`afterimage`·`capture-commit`
   - 동작(`run`): `capture.toDraw`·`capture.copyHex`·`capture.saveLast`·`draw.acceptSuggestion`·`draw.undo`·`record.*`·`pins.closeAll`
5. **원본 바이트 IPC**: `draw_export`·`capture_commit_annotated`는 Tauri 2 `tauri::ipc::Request` 원본 본문으로 RGBA를 받는다(base64 없음). 주석은 본문 앞 `[머리 길이 u32 LE][머리 JSON][RGBA]`. **WKWebView(맥)에서 원본 본문으로 오는지 확인 필요** — postMessage 대체 경로로 오면 명령이 거부된다(§2.1).
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

### 2.2 맥 트랙 원래 순서 — M9·M10·M17·M-sign
- **M9 AX 요소 색인**(P0, 1.5일): 시안 [I-mac.md](../plans/2026-11-26/I-mac.md) §2.1. Windows UIA 일꾼(`ink-platform-windows/src/uia.rs`)과 MSAA 대체(`msaa.rs`)의 구조·시간 예산을 본보기로.
- **M10 권한 스마트 흐름**(1일): I-mac.md §2.2. 권한 허용은 사용자에게 부탁(직접 바꾸지 않음).
- **M17 첫 실행 시트 F키 한 줄**(사용자 결정 #9): "fn+F1 또는 키보드 설정의 '표준 기능 키'" — 04 말투로 짧게. 브랜드 세션이 고친 `FirstRunSheet.svelte`에 덧붙이기.
- **M-sign**: 자체 서명 dmg·업데이트 뒤 권한 유지(I-mac.md §2.3).
- 확인만 남은 M3~M6·M8·M11은 이전 문서 §2.1 표대로.

### 2.3 맥에서 새로 만들 어댑터 — M14·M13·M15 (큰 일)
Windows 구현을 본보기로 같은 포트에 맥 구현을 꽂는다. 공통 계약은 바꾸지 않는다.
- **M14 F2 창 캡처**(0.5일): Windows `wgc_window.rs`의 흐름(미리 준비 → 확정 → 첫 프레임 시간 제한 → 대체 경로 → 보호 화면 = RGB 정확히 0 판정 `ink-domain`)을 ScreenCaptureKit 창 필터로. 둥근 모서리 귀만 투명(사용자 결정, R-OUT-3과 일치).
- **M13 실시간 확대**(1.5일): PRD R-ZOOM-1 맥 = **SCStream → Metal**. 포트 `Magnifier`, Windows `magnifier.rs`(전용 스레드·미리 만들기·우리 창 제외)·`live_zoom.rs`. 사용자 결정: 확대 중 전역 키는 Esc만, 휠 = 배율(1.25배에서 멈춤), Shift+휠은 아래 앱으로.
- **M15 F3 녹화**(1.5일): 포트 `Recorder`, Windows `recorder.rs`(MF 직접, 하드웨어 H.264, 1080p30 **5 Mbps** — 사용자 결정). 맥 = SCStream + AVAssetWriter(H.264). 창 클릭 = 창 추적(사용자 결정 #6). 자동 중지·임시 파일 복구는 공통(`record.rs`).

### 2.4 맥에서 해도 되는 플랫폼 무관 일
- 브라우저 미리보기 확인: `npm run ui:dev` → `?draw=1`(옵션 `&board=1`·`&apply=auto`), `?screen=island`, `?screen=settings&tab=draw`.
- **[할 일] 결과 섬이 떠 있는 동안 Alt+1 = [판서로]**(메인 결정 — 키캡 `Alt 1`과 맞춤). `service.rs` 단축키 판정에서 결과 섬 상태를 보고 `capture.toDraw`로.
- **[할 일] 판서 중 트레이로 실시간 확대를 켜면 판서를 끝내지 않는 버그**(`start_zoom` — 이어받기 일꾼이 발견).
- **[할 일] 판서 엔진 확장**(캡처 주석 일꾼 요청, 지금은 바깥에서 우회): 고정 도형 도구, 모자이크 도구, `setWidth(px)`, `drawMosaic` 박스 평균(미리보기 = Rust 결과 화소 일치), `DrawCanvas` 투명 바탕·보기 원점 옵션.
- **[할 일] 여러 모니터에 걸친 주석 동기화**(`captureAnnotSync`) — 지금은 툴바 모니터 창에서만 그림.
- 단계 7.7 디자인 관문, 문서 정리.

---

## 3. Windows에서 할 일 (학교 PC로 돌아갔을 때)

### 3.1 화면 확인 묶음 — 사용자에게 미리 묻고 한 번에(합계 약 3시간)
1. **인식**(1.7~2시간): `bench/recog/RUNBOOK-windows.md` 순서대로 — RB-15·16 실제 창(`cargo test -p ink-platform-windows --test rb15_16 real:: -- --ignored --test-threads=1`), DPI·MSAA 실측(125%/150% PC 필요 — 배율 변경은 사용자 승인), RB-20 멈춘 앱, 사례 17종(RB-01~14·17·18·19로 해석)·기준선, G3, Snipaste 대조, 붙여 넣기.
2. **판서 G5**: 펜 지연 로그 `pen latency n=.. p50=.. p95=..`(소프트웨어 p95 ≤ 25 ms), 240 fps 촬영 p95 ≤ 50 ms, 진입 `mode enter draw` p95 ≤ 150 ms.
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

(마감 커밋 뒤 채움)
