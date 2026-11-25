# 아이캔 줌잇 0.7.2.2 분석

2026-11-25 · 방법: [README](README.md)의 클린룸 원칙 — 폴더 파일, 동봉 단축키 PDF, 레지스트리 설정(계정 값 제외), rea 정적 메타데이터(.NET 어셈블리 정체·P/Invoke·형식 이름·문자열 목록·매니페스트). 메서드 본문 디컴파일 없음. 이 세션에서 앱을 실행하지 않았다.
대상 SHA-256 `02366cb66c9eb1ebd84bf0738a8ec281a8955ab2a40169af06a4ca9d696dd96d` · rea Evidence `ev_4ea939f6241bd5a422b11d7421d53fb1071f8a824c95dc861b0261e1b8930ae5`(P/Invoke)

## 1. 정체

- **VB.NET · WPF · .NET Framework 4.0**(AnyCPU). 참조: PresentationFramework·PresentationCore·WindowsBase·System.Xaml·System.Drawing·Microsoft.VisualBasic. 강력한 이름 서명 없음.
- 실행 파일 1개(1.2 MB) + `en-US\ICanZoomit.resources.dll`(영어 위성 리소스) + `단축키.pdf`·`Shortcut.pdf`.
- 매니페스트: `dpiAware=true`(시스템 DPI 인식만, Per-Monitor 아님) · `asInvoker`.
  → **배율이 다른 두 번째 모니터에서는 Windows가 비트맵을 늘려 그려 흐려지고, 좌표가 어긋날 수 있는 구조**다. 아이디어-고도화-1차 2절 불만 7번("배율이 섞이면 좌표가 밀린다")의 원인 후보. S10 시나리오로 실측 필요.

## 2. 구조 (형식 이름에서 읽은 것)

| 영역 | 형식 | 해석 |
|---|---|---|
| 진입·창 | `MainWindow` `frmZoom` `frmZoomit_Win11` `frmCanvas` `frmTimer` `frm_TimerAdvanced` | 확대 창, Windows 11용 별도 확대 창, 판서 캔버스, 타이머 |
| 칠판 | `ctlBlackBoard` `ctlBlackBoard_Number` `ctlBlackBoard_Style` `clsBlackboard_StructureData` `clsPageWorker` | 칠판 + 페이지 관리 + 배경 스타일 |
| 판서 | `Stroke_Line` `Stroke_Ellipse` `Stroke_Tetragon` `Stroke_Check` `Stroke_Number` `textStroke` | WPF Ink의 `Stroke`를 도형·체크·번호 도장·텍스트로 확장 [추정] |
| 실행 취소 | `CommandStack` `CommandItem` `StrokesAddedOrRemovedCI` `SelectionMovedOrResizedCI` `ChangeAttributes` | 명령 패턴: 획 추가·삭제 / 선택 이동·크기 / 속성 변경 단위로 되돌리기 |
| 단축키 | `HotkeyClass` `KeybordHook` `LowLevelKeyboardProc` `KBDHookStruct` `Ctl_HotKey` `frm_Shotkey` `frm_Canvas_Shotkey` | **저수준 키보드 훅**으로 전역 단축키, 단축키 재지정 화면 |
| 캡처 | `screenCaptureGDI` `frm_Capture_Select` `MouseIcon_Info` `CURSORINFO` `ICONINFO` | GDI 캡처, 영역 선택, 커서 모양 합성 |
| 툴바 | `toolBar` `start_toolbar` `color_toolbar` `monitor_toolbar` `slide_toolbar` `toolbarButton` `toolbarGroupButton` | 왼쪽 툴바, 모니터 선택, 슬라이더 |
| 설정 | `MySettings` `Registry_Import_Export` `DataParsing` | 레지스트리 설정 + 내보내기·가져오기 |
| 사용자 | `frm_SelectUsers` `frm_SwitchUsers` `Ctl_User` | **여러 사용자 프로필** 전환(교사 여러 명이 한 PC 공유) |
| 기타 | `HighLight_Info` `Cursor_SetupInfo` `touchEventToChengeEditMode` `Transformation` | 화면 강조, 커서 표시, 터치로 모드 전환, 확대 변환 |

## 3. 가져다 쓰는 OS API (P/Invoke, 관찰)

| DLL | 함수 | 해석 |
|---|---|---|
| **Magnification.dll** | `MagInitialize` `MagSetWindowSource` `MagSetWindowTransform` `MagShowSystemCursor` `MagUninitialize` | **실시간 확대 = Windows 돋보기 컨트롤 창**. 원본 영역과 배율을 넘기면 OS가 확대해 그린다. |
| gdi32 | `BitBlt` `CreateCompatibleDC` `CreateCompatibleBitmap` `CreateBitmap` `SelectObject` `DeleteObject` | **정지 확대·판서 배경 = GDI 화면 복사** |
| user32 | `SetWindowsHookEx` `CallNextHookEx` `UnhookWindowsHookEx` | 전역 단축키(저수준 키보드 훅). `RegisterHotKey`는 쓰지 않는다. |
| user32 | `GetCursorInfo` `CopyIcon` `GetIconInfo` `GetCursorPos` `SetCursorPos` | 커서를 캡처에 그려 넣기, Space = 커서 가운데로 |
| user32 | `SetLayeredWindowAttributes` `SetWindowLong(Ptr)` `SetWindowPos` `CreateWindowExW` | 투명·최상위 창, 돋보기 창 만들기 |

## 4. 설정 (레지스트리, 관찰)

- 위치: `HKCU\Software\ICanZoomit\<프로필 이름 Base64>\` — 기본 프로필 `ZGVmYXVsdA==` = "default". 하위 키 `blackboard`. 최상위에 `installpath`·`ver`.
- 자동 실행: `HKCU\...\CurrentVersion\Run`. 로그 폴더 `\ICanZoomit_Log`.
- 값(이 PC): 첫 배율 `InitZoom=1.75` · 강조 `HighLight_Radius=120`, `HighLight_opa=0.72` · 펜 굵기 `ph=3`, 형광펜 `mh=15` · 글꼴 `굴림 20 굵게` · 획 보정 `IsFitToCurve`, 직선 보정 `IsNotLineCalibration` · `esc_IsExit=1` · 툴바 `toolbar_IsHoverMode=yes`, `toolbarTargetMonitor=2`, `toolbarPos` · 터치 `touchIsEnabled/IsMove/IsZoom/Distance` · 커서 강조색 3개(`cursor_color01~03` + 불투명도) · 단축키 `skeyZoom/Draw/Timer/LiveZoom/LiveZoomInvert`.

## 5. 기능 (동봉 단축키.pdf)

| 구분 | 단축키 |
|---|---|
| 실행 | Ctrl+1 확대 · Ctrl+2 판서 · Ctrl+3 타이머 · Ctrl+4 실시간 확대(실시간 확대는 **확대·축소만**) |
| 확대 | ↑↓ = 휠 확대·축소 · Ctrl+↑↓ = 펜 굵기 |
| 기능 | Space 커서 가운데 · Tab 원 · Ctrl 사각형 · Shift 직선 · Shift+Ctrl 화살표(+Alt 점선) · Ctrl+Z/Y · **Ctrl+Shift+R 직전 판서 가져오기** · Ctrl+C 화면 복사 · **Ctrl+Shift+C 영역 선택 복사** · Ctrl+S 저장 · Shift+K 빈 칠판 · **Shift+Ctrl+F12 색 반전**(실시간 확대에서도) · **H 화면 강조**(Ctrl+휠 원 크기, Shift+휠 배경 불투명도) |
| 판서 | P 펜 · M 형광펜 · S 선택 · T 텍스트(Ctrl+T 글꼴) · D 지우개(획·도형 단위) · E 전체 삭제 · **F6 체크 아이콘 · F7 번호 아이콘**(넣을 때마다 1씩 증가, 숫자키로 다음 번호 지정) |
| 타이머 | ↑↓ 1분 · ←→ 10초 · Ctrl+↑↓ 글자 크기 · Space 일시정지 |
| 색 | W 흰색 · K 검정 · R 빨강 · Y 노랑 · G 초록 · O 주황 · B 파랑 · L 라임 |

## 6. 네트워크·개인정보 관련 문자열 (정적 관찰 — 실제 전송은 관찰하지 않음)

어셈블리 문자열 목록에 다음이 들어 있다: 버전 확인 주소 `http://ver.icannote.com/versionInfo/icanzoomit_ver.asp`(HTTPS 아님), 경로 `/member/userLog.aspx`, WMI 질의 `SELECT * FROM Win32_NetworkAdapterConfiguration WHERE IPEnabled='TRUE'`, `icanzoomit_update.exe`.
→ 업데이트 확인과 사용 기록 전송, 네트워크 어댑터 정보 조회를 **할 수 있는 코드가 있다는 정도까지만** 말할 수 있다 [추정]. 언제·무엇을 보내는지는 확인하지 않았다(확인하려면 사용자 승인 아래 네트워크 관찰이 필요).
→ screen-ink의 "외부 전송 0"은 문서 약속이 아니라 **정적으로 검증 가능한 성질**로 만든다(PRD R-NET-1·R-NET-2).

## 7. 강점·약점 정리

| 강점(가져올 것) | 약점(기회) |
|---|---|
| 한국 교실에 맞춘 단축키·색 한 글자 키, 체크·번호 도장, 화면 강조, 색 반전 | 시스템 DPI 인식만 → 혼합 배율에서 흐림·좌표 문제 가능 |
| 실행 취소가 획·선택·속성 단위로 정교함 | 실시간 확대 중 판서 불가(PDF 명시) — Magnification API 구조상 확대 창 위 입력이 어렵다 |
| 여러 사용자 프로필, 설정 내보내기·가져오기 | 전역 키를 저수준 훅으로 잡음 → 훅 시간 초과 시 OS가 훅을 끊을 수 있고 백신 휴리스틱에 걸리기 쉬움 [추정] |
| 터치 확대·이동, 툴바 호버 모드(전자칠판) | GDI 캡처 → 일부 하드웨어 가속·보호 화면이 검게 찍힐 수 있음 [추정] |
| Ctrl+Shift+R로 직전 판서 되살리기(판서 보존의 원형) | 판서 중 스크롤 불가, 캡처·핀 없음, Windows 전용, 무서명 |

## 8. screen-ink 요구사항으로 옮긴 것

| PRD ID | 내용 | 근거 |
|---|---|---|
| R-DPI-1 | Per-Monitor V2 매니페스트 + 모니터별 좌표 변환 단위 시험 | 1절 |
| R-KEY-1 | 전역 키는 `RegisterHotKey`(tauri global-shortcut), 저수준 훅은 쓰지 않는다 | 3절 |
| R-ZOOM-1 | Windows 실시간 확대 1안 = Magnification API(검증된 경로), 2안 = WGC → GPU(판서 겹치기가 필요해질 때) | 3절, 7절 |
| R-DRAW-1 | 실행 취소 단위: 획 추가·삭제 / 이동·크기 / 속성 변경 | 2절 |
| R-DRAW-2 | 직전 판서 다시 불러오기(Ctrl+Shift+R 대응, 1차는 마지막 1장 보존) | 5절 |
| R-DRAW-3 | 화면 강조(H)·색 반전은 1차 후보에서 P1로, 체크·번호 도장은 P1 | 5절 |
| R-SET-1 | 설정 내보내기·가져오기(JSON 파일), 프로필은 후속 | 4절 |
| R-NET-1·2 | 네트워크 0 + 빌드에 네트워크 기능을 넣지 않는다(정적 검사로 확인) | 6절 |
