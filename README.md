# screen-ink (가칭)

발표자를 위한 **화면 필기·확대·스마트 캡처** 데스크톱 앱. 아이캔 줌잇(판서·확대)과 Snipaste(스마트 캡처)의 장점을 한 앱으로 합친다.
Windows 먼저, 이어서 macOS. 한국어 우선 + English. 무료(이후 MS Store).

> 상태: **PRD v0.2(벤치마크 반영) 검토 대기, 코드 없음** (2026-11-25) → [docs/PRD.md](docs/PRD.md)
> 원본 문서(구글 독스): https://docs.google.com/document/d/1Uuzck1KYOipR9-J85CR14jo1mxsjXSucKD38nU61EB8/edit

## 1차 프로토타입 — 핵심만

| 단축키 | 기능 |
|---|---|
| **F1** | 범위 캡처 — 창·UI 요소 자동 인식, 드래그로 직접 지정, 복사·저장 |
| **F2** | 창 캡처 — 가려진 부분까지 그 창만 |
| **F3** | 화면 녹화 — 범위·모니터 선택, 다시 F3이면 MP4 저장 |
| **Alt+1** | 판서 모드 — 화면을 고정하고 필기, 휠로 확대·축소 |
| **Alt+2** | 화면 확대 모드 — 움직이는 화면 그대로 실시간 확대, 휠로 배율 |
| **Alt+3** | 칠판 — 흰색(W)·검정(K) 빈 칠판 |

- **그리기 단축키**(아이캔 줌잇 체계): 펜 P · 형광펜 M · 텍스트 T · 선택 S · 지우개 D · 전체 삭제 E · 원 Tab · 사각형 Ctrl · 직선 Shift · 화살표 Ctrl+Shift(+Alt = 점선) · 색 R/Y/G/O/B/L · Ctrl+휠 = 굵기.
- **듀얼 모니터**: 커서가 있는 모니터에서 작동, 배율이 섞여도 정확하게.
- **툴바 없음**: 실행하면 트레이에만 상주. 트레이 아이콘 → 설정 창(단축키 재지정·그리기·캡처/녹화·일반).
- **없는 것**: DB·로그인·서버·통계. 전부 오프라인.
- **디자인**: Tidy-Task-Local처럼 아기자기하고 깔끔하게(파스텔 테마, 둥근 카드, 짧게 떴다 사라지는 말풍선).

## 기술

Tauri 2 + Svelte 5 + Tailwind 4(Tidy-Task-Local과 같은 구성). 캡처·영역 인식·확대·녹화는 Rust에서 OS 네이티브 API로 처리한다.
Windows: UI Automation · Windows.Graphics.Capture · Magnification API · Media Foundation / macOS: ScreenCaptureKit · 손쉬운 사용(AX) · AVFoundation.

- 난이도 **중상**, 약 15~18작업일 [추정]. 어려운 곳: F3 녹화, 맥 실시간 확대, 배율 섞인 듀얼 모니터.
- 개발 순서: **Windows 먼저**(학교 PC) → macOS(맥미니 4K+FHD 듀얼).

## 후속(1차에서 뺀 것)

캡처 핀 · 캡처 주석 편집 · 스크롤 캡처 · OCR·QR · 라이브 판서(클릭 통과) · 판서 보존·스크롤 따라가기 · 타이머·교실 위젯 · 명령 팔레트 · 시스템 소리 녹음 · GIF · MS Store·공증 배포.

## 문서

- [docs/PRD.md](docs/PRD.md) — **1차 프로토타입 PRD v0.2**: 요구사항 ID·합격 기준·측정 목표·기술 근거·벤치마크 대조표
- [docs/benchmark/](docs/benchmark/README.md) — 벤치마킹 방법(클린룸·지표·시나리오 카드)과 대상별 분석: [Snipaste 2.11.3](docs/benchmark/snipaste-2.11.3.md) · [아이캔 줌잇 0.7.2.2](docs/benchmark/icanzoomit-0.7.2.2.md) · [ZoomIt 0.94.1](docs/benchmark/zoomit-powertoys-0.94.1.md)
- [bench/](bench/) — 측정 스크립트(`snipaste-log-stats.ps1` · `sample-process.ps1`)와 결과 CSV
- [docs/WINDOWS-SETUP.md](docs/WINDOWS-SETUP.md) — Windows PC 개발 환경 준비
- [docs/아이디어-고도화-1차.md](docs/아이디어-고도화-1차.md) — 벤치마크·기회·기능 아이디어·플랫폼·배포·13절 프로토타입 목표 v0.1(구글 독스 사본)
- [docs/research/](docs/research/) — 조사 원문(Antigravity 딥서칭 · Codex 아이디어 · Claude 사실 확인 · 오픈소스 조사)
