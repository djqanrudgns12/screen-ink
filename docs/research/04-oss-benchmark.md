# 오픈소스·커뮤니티 벤치마크 (04)

2026-11-25 · 조사: **Antigravity**(Orchestrator Neon — 병렬 하위 에이전트 5개: 캡처·핀 / 확대·판서·칠판 / 녹화 / Rust·Tauri 부품 / 커뮤니티, 웹 조사) · 클론·라이선스 확인·로컬 검증·통합: **Claude**(Opus 5.5)
참고 코드 위치: `C:\Users\User\Desktop\vibecoding\screen-ink-refs\`(저장소 바깥, 읽기 전용) — 색인 `screen-ink-refs\INDEX.md`

표기: **✔** 로컬 클론에서 파일·줄을 열어 확인 · **◇** Antigravity 웹 조사(로컬 미검증) · **✘** 확인해 보니 다름(고침)

## 0. 결론 요약

1. **허용 라이선스만으로 1차 기능을 모두 짤 수 있다.** 받은 29개 저장소는 전부 MIT·Apache-2.0·MS-PL·BSD다(Cap은 MIT인 `scap-*` 크레이트만 받음). GPL·AGPL(ShareX·Flameshot·OBS·OpenBoard·Rnote·snow-shot 등)은 받지 않고 링크와 아이디어만 남겼다.
2. **기준 구현은 ZoomIt(MIT)**: 실시간 확대 = Magnification API, 확대 중 판서 = 계층 창 + `MagSetWindowFilterList`, 녹화 = WGC + WinRT `MediaTranscoder`, 자기 창 제외 = `WDA_EXCLUDEFROMCAPTURE`, 단축키 = `RegisterHotKey` + `MOD_NOREPEAT`. macOS판도 Microsoft가 공식으로 냈다(`microsoft/Zoomit-Mac`, MIT).
3. **Rust 부품으로 바로 쓸 것**: `xcap`(F1, 기본 GDI·`wgc` 기능 선택), `windows-capture`(F2 창 캡처 `CreateForWindow` + F3 MP4 인코더), `uiautomation-rs`(F1 요소 인식·트리 탐색), `global-hotkey`(Tauri 단축키, 이미 `MOD_NOREPEAT`), `tauri-nspanel`(맥 전체 화면 위 오버레이).
4. **같은 스택의 실전 앱 `ifer47/markeron`(Tauri 2 화면 판서, MIT)**: WebView2가 오래 쉬거나 GPU가 재활용되거나 DPI가 바뀌면 투명 배경을 잃고 검게 변한다 → 보일 때마다 투명도를 다시 건다. "미리 만들어 숨겨 둔 오버레이" 설계의 직접적인 위험이라 PRD에 넣었다(R-SYS-6).
5. **커뮤니티 불만 상위**: 비공개·중국발 캡처 앱의 프라이버시 걱정, 판서 앱 구독 전환(Epic Pen), 혼합 DPI 좌표 밀림, ShareX의 복잡함, ZoomIt의 한글 IME 지연·개체 선택 없음, 학교 PC 관리자 권한·백신 오탐, 단축키 충돌, 녹화 시 판서 누락 ↔ 툴바 노출. PRD의 네트워크 0·PMv2·IME 보호·충돌 안내·녹화 제외 규칙과 그대로 맞물린다.

## 1. 검수 결과 (Claude)

| # | Antigravity 보고 | 로컬 확인 결과 | 근거 |
|---|---|---|---|
| 1 | xcap `PrintWindow` 플래그 "0→3 순 폴백" | ✘ **2(`PW_RENDERFULLCONTENT`) → 0 → 4 → `BitBlt`** 순. DWM 크기 조절 테두리는 잘라내고, 표준 대화 상자(`#32770`)는 자르지 않음 | `xcap/src/windows/gdi.rs:167~179, 211, 225` |
| 2 | xcap 모니터 캡처 = GDI | ✔ + 보충: **기본은 GDI `BitBlt`, `wgc` 기능을 켜면 WGC**. WGC 창 캡처는 대상 창·자기 프로세스의 DPI 인식 여부로 배율을 보정 | `xcap/src/windows/capture.rs:29~112` |
| 3 | windows-capture = "Media Foundation H.264(`MFVideoFormat_H264`)" | ✘ **WinRT `MediaTranscoder` + `MediaEncodingProfile`**(MF는 그 안쪽). ZoomIt과 같은 경로. `VideoEncoder::new/send_frame/send_frame_with_audio/finish` | `windows-capture/src/encoder.rs:20, 918, 990, 1232, 1266, 1370` |
| 4 | windows-capture 창 캡처 `CreateForWindow` | ✔ `window.rs:371`, 모니터 `monitor.rs:448`, 노란 테두리 끄기 `SetIsBorderRequired(false)`, 커서 포함 설정 | `graphics_capture_api.rs:315~330` |
| 5 | ScreenRecorderLib `RecordingManager.cpp`의 `IMFSinkWriter` | ✘ `IMFSinkWriter`는 **`OutputManager.cpp`**(+ `CMFSinkWriterCallback.h`) | 로컬 검색 |
| 6 | global-hotkey = `HWND_MESSAGE` + `RegisterHotKey` | `RegisterHotKey`는 ✔(`windows/mod.rs:97`), **기본으로 `MOD_NOREPEAT`**(`mod.rs:78`) ✔. `HWND_MESSAGE`는 Windows 구현에서 찾지 못함 → [확인 안 됨] | `global-hotkey/src/platform_impl/windows/mod.rs` |
| 7 | ZoomIt `StartLiveZoom`/`ToggleLiveZoom`/`LiveZoomWindowProc` | ✘ 그런 이름 없음. 실제는 `g_hWndLiveZoom`·`g_hWndLiveZoomMag` 창과 `WM_USER_SET_ZOOM` 메시지 | `Zoomit.cpp:7869, 8205~8255` |
| 8 | ZoomIt `SetWindowDisplayAffinity` 위치·PMv2 설정부 [미확인] | ✔ 해결: 선택 테두리·미러 테두리·배지·웹캠 창에 `WDA_EXCLUDEFROMCAPTURE`; PMv2는 매니페스트(`dpiAwareness=PerMonitorV2`), 창별 DPI는 `GetDpiForWindowHelper` | `SelectRectangle.cpp:465`, `Utility.cpp:650`, [ZoomIt 분석](../benchmark/zoomit-powertoys-0.94.1.md) |
| 9 | xland/ScreenCapture 내부 파일 [미확인] | ✔ 해결: `App.cpp:91` `WDA_EXCLUDEFROMCAPTURE`, `Win/CutMask.cpp:41~49` `EnumWindows` + **`DWMWA_CLOAKED` 제외** + `DWMWA_EXTENDED_FRAME_BOUNDS`. UIA `ElementFromPoint`는 쓰지 않음 → **창 단위 인식만** | 로컬 검색 |
| 10 | gInk 클릭 통과 `WS_EX_TRANSPARENT` | ✔ `FormCollection.cs:585`, `FormDisplay.cs:111` (`WS_EX_LAYERED \| WS_EX_TRANSPARENT`) | 로컬 검색 |
| 11 | tauri-nspanel `can_join_all_spaces`·`FullScreenAuxiliary`·`NonActivatingPanel` | ✔ `builder.rs:178, 226~227, 485` | 로컬 검색 |
| 12 | Cap 분할 라이선스 | ✔ 루트 `LICENSE` 5번째 줄: "`cap-camera*`·`scap-*` 크레이트는 MIT". 받은 3개 크레이트 Cargo `license = "MIT"` | `CapSoftware__Cap/LICENSE:5` |
| 13 | perfect-freehand `getStroke.ts` 등 | ✔ `packages/perfect-freehand/src/` | 로컬 |
| 14 | ColorPicker·CropAndLock·Magnification 샘플 경로 | ✔ `MouseInfoProvider.cs`, `ZoomWindowHelper.cs`, `ThumbnailCropAndLockWindow.cpp:151`(`DwmRegisterThumbnail`), `MagnifierSample.cpp:225, 283` | 로컬 |
| 15 | Handy `overlay.rs`·`tray.rs` | ✔ `src-tauri/src/overlay.rs`, `tray.rs` | 로컬 |
| 16 | 커뮤니티 출처 32개 | 모두 검색 리다이렉트 주소였다 → **실제 주소로 풀었다**(5절). 그중 **T04(PixPin 추천)는 무관한 글**(r/VirtualYoutubers 뮤직비디오) → 근거에서 뺌, C02(Epic Pen 구독 반발 2)는 관련성 약함. 나머지는 글 제목 기준으로 관련 있음(본문은 열지 않음). | `curl` 리다이렉트 확인 |
| 17 | snow-shot Tauri → C++/Qt+Rust 전면 재작성·GPL, Cap의 Tauri 제거 | ◇ 웹 조사만. 로컬 미검증(GPL·AGPL이라 받지 않음) | — |
| 18 | 스타 수·최신 릴리스 날짜 | ◇ 웹 조사. 받은 저장소는 커밋 날짜로 대신 확인(2절) | — |

Antigravity 첫 실행은 클론까지 하고 문서 쓰기에서 실패했다(헤드리스 모드 `write_file` 권한 자동 거부). 권한은 바꾸지 않았고, 두 번째 실행은 웹 조사만 해서 답장으로 받았다. 첫 실행이 받은 `ifer47/markeron`은 두 번째 보고서에는 빠졌지만 같은 스택이라 중요하다(3절).

## 2. 받은 저장소 (29개, 전부 허용 라이선스)

| 폴더(`screen-ink-refs\`) | 라이선스 | 커밋(날짜) | 쓸 곳 |
|---|---|---|---|
| `microsoft__PowerToys` (sparse: ZoomIt·colorPicker·MeasureTool·CropAndLock) | MIT | 51f105c (2026-11-24) | Alt+1·Alt+2·F3 기준, 확대경·색, 창 경계, 라이브 썸네일 |
| `microsoft__Zoomit-Mac` | MIT | 45e15ac (2026-10-29) | macOS 단계 기준 |
| `microsoft__Windows-classic-samples` (sparse: Samples/Magnification) | MIT | 434f600 (2026-09-02) | Magnification API 최소 예제 |
| `ifer47__markeron` | MIT | 2e5e973 (2026-11-24) | **같은 스택(Tauri 2)** 판서 오버레이·포터블·커서 가두기 |
| `cjpais__Handy` (sparse: src-tauri/src) | MIT | ca2334c (2026-11-24) | Tauri 2 오버레이·트레이·단축키 |
| `EcoPasteHub__EcoPaste` (sparse: src-tauri) | Apache-2.0 | 5139d30 (2026-08-12) | Tauri 2 트레이·단축키·포커스 안 뺏는 창 |
| `ahkohd__tauri-nspanel` | MIT OR Apache-2.0 | 3ffa93e (2026-10-25) | 맥 NSPanel |
| `tauri-apps__global-hotkey` · `tauri-apps__tray-icon` | Apache-2.0 OR MIT | 41b011d (2026-11-09) · 4d125a5 (2026-11-24) | 전역 키 · 트레이 |
| `nashaofu__xcap` | Apache-2.0 | b0975cb (2026-11-20) | F1 정지 캡처 |
| `NiiightmareXD__windows-capture` | MIT | 6ab0086 (2026-11-14) | F2 창 캡처 · F3 인코더 |
| `CapSoftware__scap` | MIT | c03f15a (2025-08-05) | 다중 플랫폼 스트림 캡처 |
| `CapSoftware__Cap` (sparse: `crates/scap-direct3d`·`scap-screencapturekit`·`scap-targets`만) | 해당 크레이트 MIT(루트는 AGPL) | 3644164 (2026-11-25) | WGC·SCK 캡처 크레이트 |
| `AugmendTech__CrabGrab` | MIT OR Apache-2.0 | a535cfb (2024-06-14) | GPU 제로카피 스트림(Alt+2 2안) |
| `leexgone__uiautomation-rs` | Apache-2.0 | 58a4bc0 (2026-11-21) | F1 요소 인식 |
| `doom-fish__axuielement-rs` · `doom-fish__screencapturekit-rs` | MIT OR Apache-2.0 | 2e87027 (2026-10-26) · 791102a (2026-11-22) | 맥 AX · SCK |
| `xland__ScreenCapture` | MIT | 5e4713e (2026-10-14) | Snipaste류 C++ 캡처 |
| `TheJoeFin__Text-Grab` | MIT | f034501 (2026-10-26) | 혼합 DPI 전체 화면 정지 오버레이(WPF) |
| `NickeManarin__ScreenToGif` | MS-PL | a4d0a67 (2026-07-28) | 녹화 범위 UX |
| `sskodje__ScreenRecorderLib` | MIT | 7e8703c (2026-10-27) | MF `IMFSinkWriter` 녹화 |
| `geovens__gInk` · `pubpub-zz__ppInk` | MIT | 78aa853 (2025-02-11) · de97909 (2026-11-07) | Windows 화면 판서 |
| `epilande__Annotate` | MIT | b1976f2 (2026-11-24) | 맥 화면 주석 |
| `steveruizok__perfect-freehand` | MIT | 176e00f (2026-02-01) | 웹 획 외곽선 |
| `casiez__OneEuroFilter` · `MichaelMauderer__one-euro-rs` | BSD-3(Inria) · MIT | 3e2b68b (2026-11-18) · d2593d0 (2019-08-22) | 떨림 보정 |
| `flxzt__ink-stroke-modeler-rs` | MIT OR Apache-2.0 | 6155a0a (2025-05-01) | Rust 획 모델러(Google ink-stroke-modeler 포트) |

## 3. 검증된 로직 지도 (기능별)

| 기능 | 저장소 · 파일:줄 | 방식 | 판정 |
|---|---|---|---|
| **F1 창 경계** | `xland/ScreenCapture` `Src/Win/CutMask.cpp:41~49` | 진입 때 `EnumWindows`로 Z 순서 수집 → `DWMWA_CLOAKED`(다른 가상 데스크톱·숨김 UWP) 제외 → `DWMWA_EXTENDED_FRAME_BOUNDS`로 그림자 뺀 경계 | ✔ |
| **F1 UI 요소** | `uiautomation-rs` `crates/uiautomation/src/core.rs:138, 230, 901, 1449, 1472` | `element_from_point` → `get_bounding_rectangle`, 휠 = `get_control_view_walker` + `get_parent`/`get_first_child`. 캐시판(`*_build_cache`) 있음 → 커서 이동마다 COM 왕복을 줄일 수 있음 | ✔ |
| **F1 확대경·색** | `PowerToys/colorPicker` `ColorPickerUI/Mouse/MouseInfoProvider.cs`, `Helpers/ZoomWindowHelper.cs` | 커서 좌표 추적 + 주변 픽셀 확대 창 | ✔(파일) ◇(세부) |
| **F1 정지 캡처** | `xcap` `src/windows/capture.rs:29~79`, `gdi.rs:83~108`, `impl_monitor.rs:81~105` | 기본 GDI `BitBlt`(Snipaste와 같은 계열 → 이 PC 기준 가상 화면 약 58 ms 예상), `wgc` 기능 = WGC. 모니터 DPI는 `GetDpiForMonitor`를 동적 로드 | ✔ |
| **F2 가려진 창** | `windows-capture` `src/window.rs:371`, `graphics_capture_api.rs:315~330` | `CreateForWindow`로 DWM 합성 표면을 받음 → 가려진 부분까지. `SetIsBorderRequired(false)`로 노란 테두리 끔, 커서 포함 선택 | ✔ |
| F2 대체 경로 | `xcap` `gdi.rs:124~225` | `PrintWindow(PW_RENDERFULLCONTENT)` → 0 → 4 → `BitBlt`, DWM 테두리 잘라냄 | ✔ |
| F2 라이브(후속 핀) | `PowerToys/CropAndLock` `ThumbnailCropAndLockWindow.cpp:151` | `DwmRegisterThumbnail`로 창 일부를 실시간 투영 | ✔ |
| **F3 녹화** | `windows-capture` `src/encoder.rs:918~1370` · ZoomIt `VideoRecordingSession.cpp:1317~1334` | WGC 프레임 → WinRT `MediaTranscoder` + `MediaStreamSource` → MP4. 시작 직후 중지 시 트랜스코더 실패 처리(ZoomIt) | ✔ |
| F3 대안 | `ScreenRecorderLib` `ScreenRecorderLibNative/OutputManager.cpp` | MF `IMFSinkWriter` H.264, 프래그먼트 MP4(강제 종료 복구), WASAPI | ✔(파일) ◇(세부) |
| **Alt+2 실시간 확대** | ZoomIt `Zoomit.cpp:5190, 7869, 8205~8255, 8686` · `Windows-classic-samples` `MagnifierSample.cpp:225, 283` | 돋보기 창(`WC_MAGNIFIER`)에 `MagSetWindowTransform`(배율 행렬) + `MagSetWindowSource`(원본 사각형, 커서 따라 갱신). 자기 창은 `MagSetWindowFilterList(EXCLUDE)` | ✔ |
| 확대 중 판서 | ZoomIt `Zoomit.cpp:6119, 6596~6616` | 돋보기 위 계층 창, 펜 굵기 = 기본 굵기 × 배율 | ✔ |
| **Alt+1·Alt+3 판서** | `gInk` `src/FormCollection.cs:585`, `FormDisplay.cs:111`, `Root.cs` | 투명 계층 창 + 클릭 통과 토글, 다중 모니터 조율 | ✔(창 스타일) ◇(도구 메서드) |
| **Tauri 오버레이** | `markeron` `src-tauri/src/overlay.rs:29~53, 168~256, 331~398` | `set_ignore_cursor_events`, **보일 때마다 투명도 재지정**(`ensure_overlay_transparent`: `set_background_color(0,0,0,0)` + DWM blur-behind 재적용), 툴바 창을 오버레이 위로 다시 올림 | ✔ |
| 판서 모니터에 커서 가두기 | `markeron` `src-tauri/src/monitor.rs:21~71` | 판서 중 `ClipCursor`로 커서를 그 모니터 안에 가둠, 끝나면 해제 | ✔ |
| 판서 유지한 채 클릭 통과 | `markeron` `overlay.rs:24~27, 55~62` | `Preserved` 모드: 판서는 남기고 오버레이는 클릭 통과, 이때는 Esc를 전역 키로 잠시 등록(통과 창은 키를 못 받으므로) | ✔ — 후속 "라이브 판서·판서 보존"의 구현 힌트 |
| 포터블 설정 | `markeron` `src-tauri/src/portable.rs:1~27` | 실행 파일 옆에 **고유한 이름의 표시 파일**(`markeron.portable`)이 있을 때만 포터블 → 설정은 `{실행 파일 폴더}\data`. 흔한 `portable` 파일로 우연히 켜지지 않게 이름을 고유하게 | ✔ |
| **전역 단축키** | `global-hotkey` `src/platform_impl/windows/mod.rs:78, 97` | `RegisterHotKey`, 기본 `MOD_NOREPEAT` → PRD R-KEY-1은 Tauri 기본값으로 충족 | ✔ |
| **맥 전체 화면 위 오버레이** | `tauri-nspanel` `src/builder.rs:178, 226, 485` | `can_join_all_spaces` + `FullScreenAuxiliary` + `nonactivating_panel` | ✔ |
| **자기 창 제외** | ZoomIt `SelectRectangle.cpp:465` · `xland/ScreenCapture` `App.cpp:91` | 안내·선택 UI 창에만 `WDA_EXCLUDEFROMCAPTURE`, 판서 창은 녹화에 남김 | ✔ |
| **필기 보정** | `perfect-freehand` `packages/perfect-freehand/src/getStroke.ts` 외 · `OneEuroFilter` · `ink-stroke-modeler-rs` | 웹: `streamline` + 필압 시뮬레이션 외곽선 / Rust: One Euro 필터, 스프링-질량 모델러 | ✔(파일) ◇(알고리즘 세부) |

## 4. 후보 목록 (Antigravity 웹 조사 52개 요약)

최근 릴리스·스타는 ◇(웹 조사, 2026-11 기준). "받음"은 2절에 있는 저장소. GPL·AGPL·상용은 **코드 이식 금지, 아이디어만**.

| 범주 | 이름 | 라이선스 | 최근(◇) | 스택 | 쓸 곳 | 받음 |
|---|---|---|---|---|---|---|
| 캡처 | Flameshot | GPL-3.0 | v15.0.0 (2026-11) | C++/Qt | 선행 캡처 오버레이, DPI 이슈 사례 | — |
| 캡처 | ShareX | GPL-3.0 | v22.1.0 (2026-11) | C# | 창 인식·확대경(`DrawMagnifier`) | — |
| 캡처 | ksnip | GPL-2.0+ | v1.10.1 | C++/Qt | 핀·주석 UX | — |
| 캡처 | Greenshot | GPL-3.0 | v1.3.328 (2026-10) | C# | 창·컨트롤 인식 | — |
| 캡처 | xland/ScreenCapture | MIT | 2026-10 | C++ | 창 인식·확대경·녹화·OCR | ✔ |
| 캡처 | ScreenToGif | MS-PL | v2.43.2 (2026-07) | C# WPF | 녹화 범위 | ✔ |
| 캡처 | Text-Grab | MIT | v4.16.0 (2026-10) | C# WPF | 혼합 DPI 정지 오버레이 | ✔ |
| 캡처 | PowerToys(도구 모음) | MIT | v0.102.3 (2026-11) | C++/C# | MeasureTool·CropAndLock·ColorPicker | ✔ |
| 캡처 | Snow Shot | GPL-3.0+ | v1.6.3 (2026-11) | C++/Qt + Rust | Tauri → Qt 전환 교훈(◇) | — |
| 캡처 | eSearch | GPL-3.0 | v15.4.2 (2026-11) | Electron | 녹화·확대경·OCR UX | — |
| 캡처 | SnapX · XerahS | GPL-3.0 | 2025~2026 | C# / Avalonia | ShareX 크로스플랫폼 포크 | — |
| 캡처 | SunnyCapturer | MIT [루트 LICENSE 미확인] | v5.3.0 | C++/Qt | 확대경·DPI | — |
| 캡처 | Capturer | GPL-3.0 | 1.7.1 | C++/Qt | 영역·녹화 | — |
| 캡처 | Xray | GPL-3.0 | 2025~2026 | Tauri + React | 맥 UI 요소 검사 | — |
| 캡처 | MacShot | GPL-3.0 | [미확인] | Swift | 맥 CleanShot 대안 | — |
| 판서 | PowerToys ZoomIt | MIT | v0.102~0.103 | C++ Win32 | **기준 구현** | ✔ |
| 판서 | gInk · ppInk | MIT | v1.1.1 · 2026 | C# | 화면 판서 | ✔ |
| 판서 | Gromit-MPX | GPL-2.0+ | v1.9.0 (2026-03) | C/GTK | 토글 UX | — |
| 판서 | KeyCastr | BSD-3 | v0.10.5 | Obj-C | 맥 오버레이·키 표시 | — |
| 판서 | Annotate | MIT | 2025~2026 | Swift | 맥 주석 | ✔ |
| 판서 | Annotator · Glint · Sketchover | Apache-2.0 · MIT · Apache-2.0 | 2025~2026 | C# · Swift · — | 소규모 참고 | — |
| 판서 | DrawPen | MIT | [미확인] | — | 경량 전역 드로잉 | — |
| 판서 | markeron | MIT | 2026-11 | **Tauri 2** | 같은 스택 판서 앱 | ✔ |
| 칠판·필기 | OpenBoard | GPL-3.0 | v1.7.x | C++/Qt6 | 칠판·데스크톱 판서 UX | — |
| 칠판·필기 | Rnote · Xournal++ | GPL-3.0+ · GPL-2.0 | 2026 | Rust/GTK4 · C++/GTK | 브러시·도형 인식 UX | — |
| 칠판·필기 | Lorien | MIT | v0.6.0 | Godot | 벡터 칠판 구조 | — |
| 칠판·필기 | perfect-freehand · Excalidraw | MIT | 2026 · v0.18.x | TS | 획 외곽선·렌더링 | ✔ · — |
| 칠판·필기 | tldraw | 상용 커스텀 | v4~v5 | TS | 참고만(복제 금지) | — |
| 칠판·필기 | OneEuroFilter · one-euro-rs · ink-stroke-modeler-rs | BSD-3 · MIT · MIT/Apache | — | 다언어 · Rust | 떨림 보정 | ✔ |
| 녹화 | Cap | MIT(`scap-*`·`cap-camera*`) / AGPL(나머지) | cap-v0.6.3 (2026-11) | Rust | 캡처 크레이트만 | ✔(MIT만) |
| 녹화 | OBS Studio | GPL-2.0+ | v33.0.3 (2026-11) | C/C++ | WGC 창 캡처 예외 처리 | — |
| 녹화 | ScreenRecorderLib | MIT | v7.0.1 (2025-11) | C++/C# | MF 녹화 | ✔ |
| 녹화 | QuickRecorder | AGPL-3.0 | v1.6.9 | Swift | 맥 SCK 녹화 UX | — |
| 녹화 | OpenScreen | MIT | v2.0 | Electron + Rust | GPU 확대·팬 | — |
| 녹화 | ScreenArc · Kooha · Screenity | GPL-3.0 | 2025~2026 | 다양 | 자동 줌·녹화 UX | — |
| 녹화 | Kap · Captura | MIT | 비활성 | Electron · C# | 옛 UX만 | — |
| Rust·Tauri | xcap · scap · windows-capture · uiautomation-rs · CrabGrab | Apache/MIT | 2024~2026 | Rust | 캡처·인식 부품 | ✔ |
| Rust·Tauri | tauri plugins-workspace · global-hotkey · tray-icon · tauri-nspanel | MIT/Apache | 2026 | Rust | 단축키·트레이·맥 패널 | ✔(plugins-workspace 제외) |
| Rust·Tauri | x-win · active-win-pos-rs | MIT / MIT·Apache | 2026 | Rust | 창 목록 보조 | — |
| Rust·Tauri | windows-rs | MIT/Apache | v0.62.x | Rust | Win32·WinRT 바인딩 | (의존성으로 사용) |
| Rust·Tauri | Handy · BongoCat · EcoPaste · tauri-plugin-screenshots | MIT · MIT · Apache · MIT | 2026 | Tauri 2 | 실전 오버레이·트레이 | ✔(Handy·EcoPaste) |

## 5. 커뮤니티 조사 (Antigravity · 링크는 Claude가 실제 주소로 풀었음)

글 본문은 열지 않았다. "관련성"은 글 제목(주소) 기준 판정이다.

### 5.1 자주 추천되는 앱

| 앱 | 오픈소스 | 추천 이유(◇) | 출처 | 관련성 |
|---|---|---|---|---|
| ShareX | GPL | 기능·자동화 최강 | [r/software](https://www.reddit.com/r/software/comments/1fpaww0/can_you_recommend_a_program_for_screenshots/) · [HN 40650844](https://news.ycombinator.com/item?id=40650844) | 관련 |
| Flameshot | GPL | 영역 지정 + 인라인 주석 | [r/software](https://www.reddit.com/r/software/comments/1uvxk82/looking_for_screenshot_apps/) · [HN 40650844](https://news.ycombinator.com/item?id=40650844) | 관련 |
| Snipaste | 비공개 | F1 캡처 → F3 핀, 요소 감지·색 추출·포터블 | [r/windowsapps](https://www.reddit.com/r/windowsapps/comments/1y3ierl/tools3000_v144_i_combined_everything_ditto/) · [HN 40629054](https://news.ycombinator.com/item?id=40629054) | 약함(통합 도구 소개 글) · 관련 |
| PixPin | 비공개 | Snipaste 핀 + 스크롤·OCR·GIF | ~~T04~~ 무관한 글 → 근거 없음 | **제외** |
| ZoomIt | MIT | 무설치·MS 서명·LiveZoom | [r/ROGAlly](https://www.reddit.com/r/ROGAlly/comments/1ntbpd7/screen_zoom_hotkeys_for_older_eyes_howto/) | 약함 |
| gInk / ppInk | MIT | Epic Pen 구독 전환 뒤 대안 | [r/software](https://www.reddit.com/r/software/comments/1g31rrr/best_alternative_for_epic_pen/) | 관련 |
| Epic Pen | 비공개 | 라이브 판서 기준점, 구독 반발 | 위와 같은 글 | 관련 |
| 아이캔노트 | 비공개 | 한국 교사 주력 판서 | [와콤 코리아 블로그](https://wacomkoreablog.com/1397) | 관련 |
| 아이캔 줌잇 | 비공개 | ZoomIt 한글 입력 해결·포터블·툴바 | [티스토리](https://schoolforkids.tistory.com/entry/%EC%95%84%EC%9D%B4%EC%BA%94%EC%A4%8C%EC%9E%87) | 관련 |
| CleanShot X | 비공개 | 맥 캡처 완성도 | [HN 33326757](https://news.ycombinator.com/item?id=33326757) | 관련 |
| Shottr | 비공개 | 경량·빠름·픽셀 측정 | [r/macapps](https://www.reddit.com/r/macapps/comments/rm5s3w/best_cleanshot_x_alternative/) | 관련 |
| MacShot | GPL | CleanShot 대안 오픈소스 | [r/MacOS](https://www.reddit.com/r/MacOS/comments/1scbkm1/macshot_free_opensource_screenshot_tool_for_macos/) | 관련(제작자 소개 글) |
| Presentify / ScreenBrush | 비공개 | 맥 라이브 판서 | [r/macapps](https://www.reddit.com/r/macapps/comments/1eh8ymc/is_there_an_app_for_macos_that_can_annotate_a/) | 관련 |
| OpenBoard | GPL | 교실 화이트보드 | [r/Teachers](https://www.reddit.com/r/Teachers/comments/g5sg8a/looking_for_digital_whiteboard_software_that_lets/) | 관련 |
| DrawPen | MIT | 경량 전역 드로잉 | [GitHub DmytroVasin/DrawPen](https://github.com/DmytroVasin/DrawPen) | 저장소 자체 |

### 5.2 반복되는 불만 → screen-ink 대응

| # | 불만(◇) | 출처 | screen-ink 대응 |
|---|---|---|---|
| 1 | 비공개·중국발 캡처 앱의 통신·텔레메트리 걱정, Pro 페이월 | [r/windowsapps "Beware hidden Chinese screen shot tools"](https://www.reddit.com/r/windowsapps/comments/1u1aumn/beware_hidden_chinese_screen_shot_tools/) · [r/software](https://www.reddit.com/r/software/comments/180eeaz/screenshot_tool_similar_to_windows_10_snip_app/) | R-NET-1·2 네트워크 0을 검증 가능하게 |
| 2 | 판서 앱 구독 전환·기본 도구 페이월 | [r/software](https://www.reddit.com/r/software/comments/1g31rrr/best_alternative_for_epic_pen/) · [r/Competitiveoverwatch](https://www.reddit.com/r/Competitiveoverwatch/comments/1v9etk5/whats_the_name_of_this_app_a_lot_of_coaches_use/)(약함) | 1차 핵심 판서·칠판 무료 |
| 3 | 혼합 DPI·듀얼 모니터 좌표 밀림 | [r/sharex "monitor misalignment"](https://www.reddit.com/r/sharex/comments/1hubv2z/sharex_monitor_misalignment/) · [flameshot#3056](https://github.com/flameshot-org/flameshot/issues/3056) | R-DPI-1 PMv2 + 물리 픽셀 좌표 하나 |
| 4 | ShareX가 너무 복잡함 | [r/sharex](https://www.reddit.com/r/sharex/comments/14qcxap/sharex_complicated_and_convoluted_screen_grab_and/) · [HN 40650844](https://news.ycombinator.com/item?id=40650844) | 트레이 + 4탭 설정 |
| 5 | ZoomIt 한글 IME 지연, 개체 선택·이동 없음 | [티스토리](https://rgy0409.tistory.com/4892) · [r/software](https://www.reddit.com/r/software/comments/wgmao4/draw_on_your_screen_without_a_toolbar_or_your/) | R-DRAW-3·4 IME 보호, 선택 S |
| 6 | Windows 11 캡처 도구 2~3초 지연 | [r/techsupport](https://www.reddit.com/r/techsupport/comments/1r026ae/windows11_broke_my_snipping_tool_its_slow_and/) · [r/Windows11](https://www.reddit.com/r/Windows11/comments/1j98vig/fix_to_slow_snipping_tool_copy_to_clipboard/) | M1 p50 ≤ 80 ms |
| 7 | 관리자 권한 없음·포터블 필요·백신 격리 | [r/drawingtablet](https://www.reddit.com/r/drawingtablet/comments/1xm231p/best_free_screen_annotation_software_windows/) · [티스토리](https://geni23.tistory.com/333) | R-SET-1 포터블 · R-SYS-5 · 훅 없음 |
| 8 | 맥 유료 갱신 피로 | [r/MacOSApps](https://www.reddit.com/r/MacOSApps/comments/1swqw0n/i_was_paying_29year_for_cleanshot_x_i_built_a/) · [r/MacOS](https://www.reddit.com/r/MacOS/comments/1scbkm1/macshot_free_opensource_screenshot_tool_for_macos/) | Win → macOS 무료 |
| 9 | 전역 단축키 충돌 | [r/PowerToys "ZoomIt functions being activated with the wrong…"](https://www.reddit.com/r/PowerToys/comments/1o5zqsp/zoomit_functions_being_activated_with_the_wrong/) · [티스토리](https://schoolforkids.tistory.com/entry/%EC%95%84%EC%9D%B4%EC%BA%94%EC%A4%8C%EC%9E%87) | R-KEY-2 충돌 감지·안내 |
| 10 | 녹화 시 판서 누락 ↔ 제어 UI 노출 | [OBS 포럼 "Epic Pen annotations not showing on recordings"](https://obsproject.com/forum/threads/epic-pen-annotations-not-showing-on-recordings.173470/) · [r/obs](https://www.reddit.com/r/obs/comments/1v6uh1b/obs_keeps_recording_steam_overlay/) | R-REC-2 안내 UI만 제외, 판서 포함 |
| 11 | 한국 교실 도구가 Windows 전용·앱 여러 개 병행 | [티스토리](https://jujuland.tistory.com/432) · [와콤 코리아 블로그](https://wacomkoreablog.com/1397) | 캡처+확대+판서+녹화 한 앱 |

## 6. 라이선스 정책

- **이식 가능(출처·고지 유지)**: MIT · Apache-2.0(NOTICE 파일 함께) · BSD-3. 옮긴 코드 위에 `// 출처: <저장소>@<커밋> <파일> (라이선스)` 주석, 저장소 루트 `THIRD_PARTY_NOTICES.md`에 고지 추가.
- **MS-PL(ScreenToGif)**: 파생 소스를 배포하면 MS-PL 조건을 따라야 한다 → 패턴만 참고, 코드는 옮기지 않는다.
- **분할(Cap)**: `scap-*`·`cap-camera*`만 MIT. 그 밖의 폴더는 받지 않았다.
- **금지(아이디어만)**: GPL·AGPL·상용 커스텀(ShareX, Flameshot, Greenshot, ksnip, Snow Shot, eSearch, SnapX, XerahS, Capturer, Xray, MacShot, OpenBoard, Rnote, Xournal++, Gromit-MPX, OBS, QuickRecorder, ScreenArc, Kooha, Screenity, tldraw). 이 저장소의 코드를 열어 본 사람이 같은 부분을 바로 짜지 않는다(클린룸).

## 7. PRD 반영

| PRD | 바뀐 점 | 근거(3절) |
|---|---|---|
| R-CAP-3 | 창 목록에서 **`DWMWA_CLOAKED` 창 제외** 추가 | xland `CutMask.cpp:46` |
| R-CAP-4 | UIA 캐시 질의(`*_build_cache`) 사용 | uiautomation-rs `core.rs:145` |
| R-WIN-1 | WGC `CreateForWindow` + `SetIsBorderRequired(false)`, 대체 경로 `PrintWindow(PW_RENDERFULLCONTENT)` | windows-capture · xcap |
| R-KEY-1 | Tauri `global-hotkey`가 이미 `MOD_NOREPEAT` → 별도 구현 불필요, 시험만 | global-hotkey `mod.rs:78` |
| **R-SYS-6(신규)** | 오버레이를 보일 때마다 투명 배경·DWM 투명도 재지정, 장시간 대기·DPI 변경 뒤 시험 | markeron `overlay.rs:33~53` |
| R-DRAW-6 | 보정 후보 3개를 측정으로 고름: perfect-freehand `streamline`(웹) · One Euro(웹/Rust) · ink-stroke-modeler-rs(Rust) | 3절 필기 보정 |
| 6절 정지 캡처 | xcap 기본은 GDI → 캡처 단계 55 ms를 넘으면 `wgc` 기능 또는 windows-capture 모니터 캡처로 | xcap `capture.rs:29~79` |
| R-SET-1 | 포터블 표시 파일 이름을 `screen-ink.portable`로 고유하게, 설정은 실행 파일 옆 `data\` | markeron `portable.rs` |
| 9절 위험 | Snow Shot이 Tauri에서 Qt로 옮겼다는 보고(◇) → 대기 메모리·진입 시간 측정을 단계마다 | 4절 |

## 8. 남은 [미확인]

1. ZoomIt Ctrl+2 정지 판서 진입 경로(창 미리 만들기 여부), 혼합 배율에서의 확대 좌표 계산
2. gInk·ppInk 도구별 메서드(실행 취소·지우개·도형), ScreenRecorderLib 타임스탬프 보정 세부
3. SunnyCapturer 루트 LICENSE, DrawPen 라이선스 파일, Snow Shot·Cap의 아키텍처 전환 사실관계(◇)
4. 커뮤니티 글 본문(제목 기준 판정만 함), 로그인이 필요한 인디스쿨·네이버 카페 글
5. 스타 수·최신 릴리스(받지 않은 저장소)
