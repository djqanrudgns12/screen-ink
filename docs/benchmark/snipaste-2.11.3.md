# Snipaste 2.11.3 (Windows x64 포터블) 분석

2026-11-25 · 방법: [README](README.md)의 클린룸 원칙 — 동봉 파일 목록, 로그(`splog.txt`), 설정(`config.ini`), rea 정적 메타데이터(PE import), 사용자 화면 캡처 관찰. 코드 디컴파일 없음.
대상 SHA-256 `29aac14eca91aa8fe3d44207212078599800509d3381123fd0def34d0b19b33a` · rea Evidence `ev_bfca3449667632aa246e7053d3f818cecbc2aaafcd864acc5b33b8e13fa35e08`

## 1. 정체

- **C++ · Qt 6**: Qt6Core·Gui·Widgets·OpenGL·Network·Svg·Xml·Concurrent·Core5Compat·PrintSupport, MSVC 런타임 동봉. 그 밖에 OpenSSL 3(업데이트 확인), quazip(압축), hoedown(마크다운 → 텍스트 핀 렌더링 [추정]), D3Dcompiler_47.
- **포터블 구조**: 실행 파일 옆에 `config.ini`(설정), `history\`(캡처 기록·핀 복원), `lang\*.qm` 45개(한국어 `ko.qm` 포함), `crashes\`(Breakpad), `splog.txt`(로그). 설치 없이 폴더째 옮겨 쓴다.
- 이 PC 설정: `start_on_boot=true`(시작프로그램 폴더 바로가기), `last_save_folder=Downloads`, `ask_for_confirm_on_esc=false`.
- DPI: 실행 파일에 매니페스트가 없다 → Qt 6이 코드에서 Per-Monitor V2를 켜는 방식 [추정]. 로그는 캡처마다 모니터별 `QRect · DPI · DPR`을 적는다.

## 2. 가져다 쓰는 OS API (정적 import, 관찰)

| 용도(해석) | API | 우리에게 주는 뜻 |
|---|---|---|
| 창 목록·경계 | `EnumWindows` `EnumChildWindows` `GetWindowRect` `GetWindowInfo` **`DwmGetWindowAttribute`** `IsWindowVisible` `IsIconic` **`IsHungAppWindow`** `GetClassNameW` `GetWindowTextW` `GetWindowThreadProcessId` `QueryFullProcessImageNameW` | 창 테두리는 DWM 확장 프레임(그림자 제외)으로 잡고, **응답 없는 창은 건너뛴다**(멈춘 앱 때문에 캡처가 굳지 않게). |
| UI 요소 인식 | `OLEACC!AccessibleObjectFromWindow`(MSAA), `ole32!CoCreateInstance`(COM 객체 — UIA인지 정적 import로는 [미확인]) | MSAA 경로를 함께 둔다 = UIA가 비거나 느릴 때의 대비. |
| 화면 캡처 | GDI `GetDC` `BitBlt` `CreateDIBSection` `GetDIBits`; `d3d11!D3D11CreateDevice` + WinRT `RoGetActivationFactory`(지연 로드) | GDI가 기본, D3D11·WinRT(Windows.Graphics.Capture로 보임 [추정])는 선택 경로. 로그 "Support HDR color correction: true" → **HDR 모니터 색 보정**을 따로 한다. |
| 모니터 구성 | `QueryDisplayConfig` `DisplayConfigGetDeviceInfo` `GetDisplayConfigBufferSizes` `GetMonitorInfoW` | 모니터 이름·배율·배치를 디스플레이 구성 API로 정확히 읽는다. |
| 단축키·입력 | **`RegisterHotKey`**/`UnregisterHotKey`, `SetWindowsHookExW`, `GetAsyncKeyState`, `SendInput` | 전역 키는 `RegisterHotKey`, 훅은 보조(캡처 모드 중 입력 [추정]). |
| 포커스 | `AttachThreadInput` `SetFocus` `SetActiveWindow` `GetForegroundWindow` | 오버레이가 키 입력을 확실히 받게 포커스를 가져온다. 로그에 실패 1회("Focus cannot be set: 액세스가 거부되었습니다"). |
| 세션·보안 화면 | `WTSRegisterSessionNotification` `OpenInputDesktop` `GetThreadDesktop` `GetUserObjectInformationW` | 잠금·해제·절전을 감지하고, UAC·잠금 화면(보안 데스크톱)에서는 캡처를 시도하지 않는다 [추정]. 로그 "System suspended/resumes". |
| 기타 | `PlaySoundW`(효과음), 클립보드 API, `SHGetKnownFolderPath`, 레지스트리 읽기·쓰기, `AdjustTokenPrivileges` | — |
| **없는 것** | `SetWindowDisplayAffinity` 없음 | 자기 창을 캡처에서 빼는 대신 **먼저 화면을 찍고 나서 오버레이를 띄운다**(아래 3절 순서). |

## 3. 로그로 본 F1 캡처 파이프라인

한 번의 캡처(2026-11-25 09:20:40, 모니터 2대):

```
.020 Snip request → Snipper: preparing...
.020 Desktop: QRect(0, 0, 3840 x 1080)          ← 가상 화면 전체
.082 Background size: QSize(3840, 1080)          ← 전체를 한 장으로 정지 캡처 (62 ms)
.082 Screens: 2 / 모니터별 QRect·DPI·DPR
.094 Focus set → preparation done
.098 Snipper: shown → focus in
.099 Snipper: ready [79 ms]
... 사용자 선택 ...
Screenshot (1920, 0, 1920 x 1032) destination: none   ← 오른쪽 모니터의 작업 영역(작업 표시줄 제외)
```

이 PC의 851회 집계(`bench/snipaste-log-stats.ps1`, 단계 분해는 같은 로그의 시각 차이):

| 구간 | p50 | p90 | p95 | 최대 |
|---|---|---|---|---|
| **전체**: 요청 → 준비 완료 | **75 ms** | 126 ms | 188 ms | 8048 ms |
| ① 가상 화면 정지 캡처(preparing → Background) | 58 ms | 85 ms | 113 ms | — |
| ② 창 표시·포커스(Background → ready) | 17 ms | 41 ms | 70 ms | — |
| 앱 시작(Launch time, n=70) | 1383 ms | 2664 ms | 2792 ms | 8639 ms |

**읽을 점**: 진입 시간의 약 3/4이 **전체 화면 캡처**다. 오버레이 창은 이미 만들어 두고(hide → show) 보여 주기만 해서 17 ms면 된다. screen-ink가 p50 80 ms를 넘지 않으려면 캡처 단계를 55 ms 안에 끝내야 한다(PRD R-CAP-2).

## 4. 실제 사용 패턴 (이 PC, 2026-08-20 ~ 11-25)

| 캡처 결과 | 횟수 | 비율 |
|---|---|---|
| 클립보드 복사 | 613 | 72% |
| 취소(Esc) | 117 | 14% |
| 핀(화면에 붙이기) | 74 | 9% |
| 파일로 저장(대화 상자) | 47 | 6% |

→ **F1의 기본 동작은 "복사"가 맞다.** 핀이 9%로 두 번째로 쓰이는 결과라 1차 뒤 첫 후속 기능 후보로 둔다. 취소 14%는 "잘못 눌렀거나 범위가 마음에 안 듦" — 직전 범위 다시 캡처(Snipaste는 캡처 기록 "scene saved")가 이 비율을 줄인다.

## 5. 상주·네트워크·안정성

- 대기 자원(30초 표본, `bench/results/2026-11-25-school-pc-snipaste-idle.csv`): 전용 메모리 57.6 MB · 작업 집합 74.6 MB · CPU 0% · 핸들 499→495.
- 시작할 때 핀을 되살린다("Pasters restored", "To restore: N") — 핀을 껐다 켜도 유지.
- **업데이트 확인이 학교망에서 계속 실패**: "SSL 악수 실패" 경고 51회 + 오류 44회(70번 실행 중). 학교망의 SSL 검사·차단으로 보인다 [추정]. 사용자에게는 보이지 않지만, 네트워크를 쓰는 기능은 학교에서 깨질 수 있다는 실증이다.
- 정상 종료 41/70 — 나머지는 PC 종료 등으로 강제 종료 [추정]. 설정·핀을 수시로 저장해야 한다.
- OpenGL 가속 기본 꺼짐("OpenGL acceleration: false"), 프로세스 우선순위 조정 기록.

## 6. 화면 관찰 (사용자 캡처, 2026-11-25)

- 선택 중 **확대경**: 커서 주변 픽셀 확대 + 십자선, 가상 화면 좌표 `(2029, 684)`, 색 `255, 221, 125`, 안내 "[C] 색상복사 · [Shift] RGB/HEX 값 바꾸기".
- **크기 배지**: 선택 범위 왼쪽 위 `1920 × 1032 px`.
- 선택 뒤 **오른쪽 아래 도구 막대**: 도형 · 선/화살표 · 펜 · 형광펜 · 모자이크 · 텍스트 · 지우개 · 실행 취소 · 다시 실행 | 닫기 · 핀 · 저장 · 복사.
- 작업 표시줄을 뺀 모니터 작업 영역이 하나의 "창"처럼 인식된다(바탕 화면 = 1920×1032).

## 7. screen-ink 요구사항으로 옮긴 것

| PRD ID | 내용 | 근거 |
|---|---|---|
| R-CAP-1 | 단축키 순간 가상 화면 전체를 정지 캡처 → 미리 만든 오버레이에 깐다(캡처 먼저, 표시 나중) | 3절 순서 |
| R-CAP-2 | 진입 p50 ≤ 80 ms · p95 ≤ 150 ms, 캡처 단계 ≤ 55 ms | 3절 집계 |
| R-CAP-3 | 창 경계는 DWM 확장 프레임, 응답 없는 창 건너뛰기, 숨김·최소화 창 제외 | 2절 |
| R-CAP-4 | UIA 우선 + MSAA 대비, 요소 질의는 시간 제한 | 2절 |
| R-CAP-5 | 확대경에 좌표·색 값, C = 색 복사(HEX), Shift = RGB/HEX 전환 | 6절 |
| R-CAP-6 | 기본 결과 = 클립보드 복사, 직전 범위 다시 캡처 | 4절 |
| R-SYS-3 | 잠금·절전·보안 데스크톱 감지, 깨어나면 캡처 장치 다시 준비 | 2절, 5절 |
| R-NET-1 | 기본 네트워크 0, 업데이트 확인은 사용자가 누를 때만, 실패해도 조용히 | 5절 |
| R-SYS-4 | 설정·상태를 바뀔 때마다 저장(강제 종료 대비), 포터블 모드는 실행 파일 옆에 설정 | 1절, 5절 |
| R-HDR-1 | HDR 모니터에서 캡처 색이 SDR로 바르게 보이게 변환 | 2절 |
