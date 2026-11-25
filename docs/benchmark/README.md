# 벤치마킹 방법 — 안정적·반복 가능·클린룸

2026-11-25 · 작성: Claude(Opus 5.5) · 대상 분석: [Snipaste 2.11.3](snipaste-2.11.3.md) · [아이캔 줌잇 0.7.2.2](icanzoomit-0.7.2.2.md) · [ZoomIt(PowerToys 0.94.1)](zoomit-powertoys-0.94.1.md)

"안정적"의 뜻: **같은 대상(버전·해시 고정) · 같은 PC 조건 · 같은 스크립트 · 같은 정의의 숫자**로 몇 번을 다시 재도 같은 결론이 나오게 한다. 느낌이 아니라 p50/p90/p95와 근거 파일로 비교한다.

## 1. 원칙

1. **클린룸**: 비공개 앱(Snipaste·아이캔 줌잇)은 ① 화면에서 보이는 동작 ② 앱이 남기는 로그·설정 파일 ③ 공개 문서·단축키표 ④ **정적 메타데이터**(가져다 쓰는 OS API, .NET 형식 이름, 매니페스트)까지만 본다. 메서드 본문 디컴파일, 코드·이미지·문구·화면 배치 복사, 실행 중 프로세스 주입·후킹·디버거 부착은 하지 않는다.
2. **오픈소스는 라이선스대로**: MIT·Apache-2.0·MS-PL(ZoomIt, gInk, ppInk, xcap 등)은 출처를 적고 로직을 참고·이식할 수 있다. GPL·AGPL(ShareX, Flameshot, OBS 등)은 기능·UX만 참고하고 코드는 가져오지 않는다. 목록과 로직 지도: [research/04-oss-benchmark.md](../research/04-oss-benchmark.md)(Antigravity 조사).
3. **근거를 남긴다**: 결론마다 rea Evidence ID, 로그 줄, 측정 CSV 중 하나를 붙인다. 관찰과 추정은 [추정]으로 구분한다.
4. **대상 고정**: 아래 표의 버전·SHA-256으로 잰다. 대상 앱의 자동 업데이트는 끈다. 대상 실행 파일은 저장소에 넣지 않는다(재배포 금지).
5. **우리 앱도 같은 자로 잰다**: screen-ink는 Snipaste와 같은 정의의 로그 줄(“오버레이 준비 N ms”, “시작 시간 N ms”)을 남긴다(PRD R-BENCH-1). 그러면 같은 집계 스크립트로 바로 비교할 수 있다.

## 2. 대상 (고정)

| 앱 | 버전 | 위치(이 PC) | SHA-256 | 기술(확인 방법) |
|---|---|---|---|---|
| Snipaste | 2.11.3 x64 포터블 | `C:\Users\User\Downloads\Snipaste-2.11.3-x64\Snipaste.exe` | `29aac14eca91aa8fe3d44207212078599800509d3381123fd0def34d0b19b33a` | C++ · Qt 6 · MSVC (동봉 DLL, rea `ev_bfca3449…`) |
| 아이캔 줌잇 | 0.7.2.2 (폴더 0.722) | `C:\Users\User\Downloads\ICanZoomit_0.722\ICanZoomit.exe` | `02366cb66c9eb1ebd84bf0738a8ec281a8955ab2a40169af06a4ca9d696dd96d` | VB.NET · WPF · .NET Framework 4.0 (rea `ev_4ea939f6…`) |
| ZoomIt | PowerToys 0.94.1 | `C:\Users\User\AppData\Local\PowerToys\PowerToys.ZoomIt.exe` | `85a52f190ea7ca176a7f15182c3d6ecdcb0e9554c307b13ded6136fc7926cd3a` | C++ Win32 · MIT 오픈소스 (rea `ev_8e32ecbb…`) |

## 3. 시험 PC 조건

| 이름 | 사양 | 화면 | 용도 |
|---|---|---|---|
| **학교 PC**(이 PC) | i5-13500 · Intel UHD 770 · 16GB · Windows 11 10.0.26200 | FHD 100% × 2, 가로 배치(가상 화면 3840×1080) | 기준값, 매일 회귀 |
| 혼합 배율 | 같은 PC, 디스플레이 설정에서 한쪽을 150%(또는 4K 150% 모니터) | 100% + 150% | 좌표·흐림·DPI 시험 |
| 맥미니 | Apple Silicon · 4K + FHD | 혼합 | macOS 단계에서 |

측정 전 체크: 전원 모드 '최고 성능' · 다른 캡처/판서 앱 종료(단축키 충돌 방지, 한 번에 한 앱만) · 백신 실시간 검사 상태 기록 · 재부팅 뒤 5분 대기 · 화면 배율·배치 스크린샷 1장 첨부.

## 4. 지표

| ID | 지표 | 정의 | 측정 방법 | 기준값(Snipaste/줌잇) | screen-ink 목표 |
|---|---|---|---|---|---|
| M1 | 캡처 진입 | 단축키 수신 → 정지 화면 오버레이가 보이고 키 입력을 받을 준비 | 앱 내부 로그(같은 정의) + 외부 하네스 | Snipaste **p50 75 · p90 126 · p95 188 ms** (n=851, 이 PC 실제 사용 08-20~11-25) | p50 ≤ 80 · p95 ≤ 150 ms |
| M2 | 시작 | 실행 → 트레이 상주·단축키 등록 완료 | 앱 내부 로그 | Snipaste p50 1383 · p90 2664 ms (n=70) | p50 ≤ 1500 ms |
| M3 | 대기 자원 | 트레이 상주 10분 평균, **자식 프로세스 포함**(WebView2) | `bench/sample-process.ps1` | Snipaste 전용 57.6 MB · 작업 집합 74.6 MB · CPU 0% | 작업 집합 ≤ 150 MB · CPU ≈ 0% |
| M4 | 판서 진입 | Alt+1(우리) / Ctrl+2(줌잇) → 정지 화면 위 첫 획 가능 | 외부 하네스 | 미측정 | ≤ 150 ms |
| M5 | 펜 지연 | 펜/마우스 이동 → 화면에 잉크 | 앱 내부 타임스탬프 + 240fps 슬로모션 20획 | 미측정 | p95 ≤ 50 ms |
| M6 | 실시간 확대 | Alt+2 화면 갱신 fps, 커서 따라가기 지연 | PresentMon(우리) · 슬로모션(대상) | 미측정 | 1080p ≥ 30 fps |
| M7 | 캡처 정확도 | 지정 범위와 결과 픽셀의 차이 | 시험 패턴(1px 격자·좌표 눈금)을 각 모니터에 띄우고 같은 범위 캡처 → 픽셀 비교 | 미측정 | 오차 0 px(혼합 배율 포함) |
| M8 | 요소 인식 | 커서 아래 강조 범위가 정답(UIA BoundingRectangle)과 맞는 비율, 갱신 지연 | 고정 시험 앱 세트(아래 6절) | 미측정 | 정확 ≥ 90%, 갱신 ≤ 50 ms |
| M9 | 안정성 | 200회 진입·종료 뒤 메모리·핸들 변화, 이벤트별 통과 | `sample-process.ps1` + 시나리오 카드 | Snipaste 핸들 499→495(30초) | 메모리 증가 < 5%, 핸들 증가 0 |

수치 표기: 항상 n, p50, p90, p95, 최대. 한 번 잰 값은 결론으로 쓰지 않는다(M1은 n ≥ 100).

## 5. 도구

| 도구 | 상태 | 하는 일 |
|---|---|---|
| [`bench/snipaste-log-stats.ps1`](../../bench/snipaste-log-stats.ps1) | 완료·실행함 | Snipaste `splog.txt` → M1·M2·캡처 목적지·경고·모니터 구성 집계(읽기 전용) |
| [`bench/sample-process.ps1`](../../bench/sample-process.ps1) | 완료·실행함 | 프로세스 트리(자식 포함) 메모리·핸들·CPU를 1초 간격 → CSV + 요약 |
| `bench/hotkey-latency` | **예정**(앱 뼈대 뒤) | `SendInput`으로 단축키 → 대상 창이 보이고 포커스를 얻는 순간까지 QPC로 측정. 세 앱 모두 같은 방식, 100회 반복 |
| `bench/patterns/grid.html` | **예정** | M7용 1px 격자·좌표 눈금·색 칸 시험 패턴(모니터마다 전체 화면) |
| rea CLI | 설치됨 | 정적 메타데이터 재확인(아래) |

rea 재확인 명령(포터블 Node 24.22.3로 실행, 대상 코드 실행 없음):

```powershell
$node = "C:\Users\User\tools\node-v24.22.3-win-x64\node.exe"; $rea = "C:\Users\User\tools\rea\node_modules\rea-agents\scripts\rea.mjs"
& $node $rea inspect-pe-imports "C:\Users\User\Downloads\Snipaste-2.11.3-x64\Snipaste.exe" --json
& $node $rea inspect-managed-native-boundaries "C:\Users\User\Downloads\ICanZoomit_0.722\ICanZoomit.exe" --json
```

## 6. 기능 대조 시나리오 카드

각 카드는 "준비 → 동작 → 기대"를 적고, 세 앱과 screen-ink에 같은 순서로 해 본다. 결과는 ○(됨) · △(부분) · ×(안 됨) · —(기능 없음)과 메모 한 줄.

| # | 시나리오 | 기대 | 보는 것 |
|---|---|---|---|
| S1 | 크롬 웹 페이지의 버튼 위에 커서 → F1 | 버튼 테두리에 맞춰 강조, 클릭하면 그 범위 | 요소 인식, M8 |
| S2 | S1에서 휠 위/아래 | 부모·자식 요소로 범위가 넓어지고 좁아짐 | 계층 이동 |
| S3 | 탐색기 창 위 F1 → Tab | 창 ↔ 요소 전환 | 창 경계(그림자 제외) |
| S4 | 두 모니터에 걸친 드래그 | 경계를 넘는 범위 선택, 결과 이미지 이음매 없음 | 가상 화면 좌표 |
| S5 | 방향키로 1px 이동 · 확대경 | 좌표·색 값 표시, C로 색 복사 | 미세 조정 |
| S6 | PowerPoint 슬라이드 쇼 중 F1 · Alt+1 · Alt+2 | 쇼 위에 뜨고 쇼가 멈추거나 꺼지지 않음 | 전체 화면 위 표시 |
| S7 | 다른 창에 가려진 창을 F2 | 가려진 부분까지 그 창만 | WGC 창 캡처 |
| S8 | Alt+1 → 휠 확대 → 판서 → Esc → 다시 Alt+1 | 확대와 판서가 함께 커짐, 판서 보존 정책대로 동작 | 줌잇 Ctrl+Shift+R 대비 |
| S9 | Alt+2 실시간 확대에서 영상 재생 | 영상이 끊김 없이 확대, 커서 따라감 | M6 |
| S10 | 혼합 배율(100%+150%)에서 S1·S4·S8 | 흐림 없음, 좌표 오차 0 | M7, DPI |
| S11 | 관리자 권한 창(예: 작업 관리자) 위 F1 · Alt+1 | 캡처는 됨, 입력 제한은 안내 | UIPI |
| S12 | 화면 잠금 → 해제, 절전 → 깨우기, 모니터 뽑기 → 꽂기 | 단축키·오버레이 정상, 핀·모드가 남은 화면으로 | 세션·모니터 이벤트 |
| S13 | UAC 확인 창이 뜬 상태에서 단축키 | 조용히 무시하거나 "보안 화면에서는 쓸 수 없어요" | 보안 데스크톱 |
| S14 | 학교망(SSL 검사)에서 하루 사용 | 네트워크 오류 알림·지연 없음 | 오프라인 원칙 |
| S15 | 표준 사용자 계정 · 포터블 폴더에서 실행 | 설치·관리자 권한 없이 모든 기능 | 학교 PC |

## 7. 결과 기록

- 원자료: `bench/results/<날짜>-<PC>-<앱>-<지표>.csv` (예: [`2026-11-25-school-pc-snipaste-idle.csv`](../../bench/results/2026-11-25-school-pc-snipaste-idle.csv))
- 요약: 대상별 문서의 "측정값" 표에 날짜·PC·n과 함께 추가. 숫자를 바꿀 때는 지우지 않고 새 줄로 쌓는다.

## 8. 하지 않는 것

- 대상 앱 디컴파일 결과 저장·인용, 대상 앱 리소스(아이콘·문구) 복사
- 실행 중 프로세스 주입(Frida)·API 후킹·디버거 부착
- 대상 앱 실행 파일·설치본 재배포
- 대상 앱 상표·이름·화면 모양 사용(아이디어-고도화-1차 10절)
