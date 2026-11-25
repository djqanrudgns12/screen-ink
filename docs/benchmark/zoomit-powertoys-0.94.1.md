# ZoomIt (PowerToys 0.94.1) 분석

2026-11-25 · ZoomIt은 **MIT 오픈소스**(microsoft/PowerToys `src/modules/ZoomIt`)라 소스 코드를 직접 읽고 로직을 참고·이식할 수 있다. 여기서는 이 PC에 설치된 실행 파일의 정적 메타데이터만 확인했고, 소스 로직 지도는 [research/04-oss-benchmark.md](../research/04-oss-benchmark.md)(Antigravity 조사)에 모은다.
대상 `C:\Users\User\AppData\Local\PowerToys\PowerToys.ZoomIt.exe` · SHA-256 `85a52f190ea7ca176a7f15182c3d6ecdcb0e9554c307b13ded6136fc7926cd3a` · rea Evidence `ev_8e32ecbbfe45358430dc6df945a4fecf961083d895069aa81788879c827ebfb7`

## 정적 관찰

| 항목 | 관찰 | 우리에게 주는 뜻 |
|---|---|---|
| DPI | 매니페스트 `dpiAware=true/PM` + **`dpiAwareness=PerMonitorV2`** | 혼합 배율의 표준 답. 아이캔 줌잇(시스템 DPI)과 대비 |
| 전역 키 | **`RegisterHotKey`** | 아이캔 줌잇의 저수준 훅 방식과 다름 |
| 자기 창 제외 | **`SetWindowDisplayAffinity`** | 녹화·캡처에 ZoomIt 창이 찍히지 않게 `WDA_EXCLUDEFROMCAPTURE` [추정 — 소스로 확인] |
| 정지 확대 | GDI `BitBlt` `StretchBlt` `SetStretchBltMode` | 확대 = 정지 화면을 늘려 그리기 |
| 그리기 | GDI+ `GdipDrawPath` `GdipSetSmoothingMode` `GdipSetPenLineCap` `GdipSetPenLineJoin` 등 45개, GDI `SetROP2` | 판서·도형 렌더링, 반전 펜 |
| 녹화·LiveZoom | WinRT `RoGetActivationFactory` `RoGetAgileReference` | Windows.Graphics.Capture 경로 [추정 — 소스로 확인] |
| 모니터 | `MonitorFromPoint` `GetMonitorInfoW` `EnumDisplaySettingsW` `ChangeDisplaySettingsExW` | 커서가 있는 모니터 기준 동작 |
| 기타 | `ClipCursor` `SetCursorPos` `SendInput` `PlaySoundW` `SetForegroundWindow` `BringWindowToTop` | 확대 중 커서 가두기, DemoType 입력 |

## 소스에서 확인한 것 (`screen-ink-refs\microsoft__PowerToys` 커밋 `51f105c` 2026-11-24, MIT)

| 질문 | 답 | 위치 |
|---|---|---|
| LiveZoom 방식 | **Magnification API 돋보기 창**(`g_hWndLiveZoomMag`). `Magnification.dll` 함수를 동적으로 불러온다(`pMagInitialize`, `pMagSetWindowSource`, `pMagSetFullscreenTransform`, `pMagSetWindowFilterList`). | `ZoomIt.h:122~154`, `Zoomit.cpp:259~267, 5190` |
| 확대 중 판서(LiveDraw) | 돋보기 창 위에 **계층 창(layered)**을 하나 더 띄우고, `MagSetWindowFilterList(mag, MW_FILTERMODE_EXCLUDE, 1, &hWnd)`로 그 창을 확대 원본에서 뺀다. 펜 굵기는 확대 배율로 환산(`g_RootPenWidth = g_PenWidth / g_LiveZoomLevel`). LiveDraw에서는 흐림·강조를 지원하지 않는다. | `Zoomit.cpp:6119, 6596~6616, 8205~8255, 8686` |
| 자기 창 제외 | `SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)`를 **선택 테두리**(`SelectRectangle`), 미러 창 테두리, 받아쓰기 배지, 웹캠 미리보기에 건다. 선택 테두리는 켜고 끌 수 있다(`SetExcludeFromCapture`). | `SelectRectangle.cpp:465`, `SelectRectangle.h:45`, `MirrorWindow.cpp:294`, `DictationBadge.cpp:101`, `WebcamPreviewWindow.cpp:110` |
| 녹화 경로 | **Windows.Graphics.Capture**(`GraphicsCaptureItem`) 프레임 → WinRT **`MediaTranscoder` + `MediaStreamSource`**(`PrepareMediaStreamSourceTranscodeAsync`)로 MP4. `IMFSinkWriter`가 아니다. 녹화 직후 바로 멈추면 트랜스코더가 실패할 수 있어 따로 처리한다. GIF는 별도 세션. | `VideoRecordingSession.cpp:1317~1334`, `GifRecordingSession.cpp` |
| 전역 키 | `RegisterHotKey`, 실시간 확대·녹화 등에는 **`MOD_NOREPEAT`**(누르고 있어도 반복 안 됨). 등록 실패 시 설정 대화 상자에서 바로 알린다. | `Zoomit.cpp:3689, 5725~5822` |
| 아직 안 본 것 | Ctrl+2 정지 판서 진입 경로와 창 미리 만들기 여부, 혼합 배율에서의 배율·좌표 계산 | [미확인] |

같은 녹화 경로를 Rust로 감싼 크레이트: `NiiightmareXD/windows-capture`(MIT) `src/encoder.rs`가 `MediaTranscoder`·`MediaStreamSource`를 쓴다 → screen-ink F3(Windows)의 1안.
