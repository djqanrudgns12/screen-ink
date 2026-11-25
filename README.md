# screen-ink (가칭)

발표자를 위한 **화면 필기·확대·스마트 캡처** 데스크톱 앱. 아이캔 줌잇(판서·확대)과 Snipaste(스마트 캡처)의 장점을 한 앱으로 합친다.
Windows 먼저, 이어서 macOS. 한국어 우선 + English. 무료(이후 MS Store).

> 상태: **PRD v3.0 최종**(단축키 확정 · 스마트 캡처 v2 · 디자인 확정: 그래파이트 + 다크 아일랜드 · 마스터플랜 약 27일), 코드 없음 (2026-11-25) → [docs/PRD.md](docs/PRD.md)
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
| Shift+F1 · Ctrl+F1 · Shift+F2 · Alt+F1 | 직전 범위 · 모니터 전체 · 활성 창 바로 캡처 · 지연 캡처 |
| Alt+Shift+1 · `` Alt+` `` · Alt+Shift+D | 직전 판서 다시 열기 · 단축키 켜기·끄기 · 창 항상 위 고정(Windows) |

단축키는 [확정본 v1](docs/shortcuts/README.md)이 처음 값이고, 전부 설정 > 단축키에서 바꿀 수 있다.

- **그리기 단축키**(아이캔 줌잇 체계): 펜 P · 형광펜 M · 텍스트 T · 선택 S · 지우개 D · 전체 삭제 E · 원 Tab · 사각형 Ctrl · 직선 Shift · 화살표 Ctrl+Shift(+Alt = 점선) · 색 R/Y/G/O/B/L · Ctrl+휠 = 굵기.
- **듀얼 모니터**: 커서가 있는 모니터에서 작동, 배율이 섞여도 정확하게.
- **툴바 없음**: 실행하면 트레이에만 상주. 트레이 아이콘 → 설정 창(단축키 재지정·그리기·캡처/녹화·일반).
- **없는 것**: DB·로그인·서버·통계. 전부 오프라인.
- **디자인**(확정): 라이트는 그래파이트(무채색 + 주황 한 점), 다크는 아일랜드(검은 유리). 화면 위 안내는 검은 알약 "섬" 하나, 아이콘마다 주황 잉크 한 방울 → [디자인 시스템](docs/prd/10-design-system.md)

## 기술

Tauri 2 + Svelte 5 + Tailwind 4(Tidy-Task-Local과 같은 구성). 캡처·영역 인식·확대·녹화는 Rust에서 OS 네이티브 API로 처리한다.
Windows: UI Automation · Windows.Graphics.Capture · Magnification API · Media Foundation / macOS: ScreenCaptureKit · 손쉬운 사용(AX) · AVFoundation.

- 난이도 **중상**, 약 15~18작업일 [추정]. 어려운 곳: F3 녹화, 맥 실시간 확대, 배율 섞인 듀얼 모니터.
- 개발 순서: **Windows 먼저**(학교 PC) → macOS(맥미니 4K+FHD 듀얼).

## 개발

```
crates/ink-domain            순수 규칙(단축키 판정·설정 v3·이름 규칙·좌표·스냅숏·모드·계측) — 시험 41
crates/ink-app               유스케이스(InkService: 바로 반영·단축키 등록 트랜잭션·되돌리기) — 포트만 앎
crates/ink-store             설정 파일(원자적 쓰기·백업 교대·손상 복구·포터블)
crates/ink-platform-windows  모니터·창 목록·GDI 정지 캡처·네이티브 정지 화면 창·시스템 사건·PowerToys 설정 키
crates/ink-platform-fake     가짜 어댑터 + 유스케이스 흐름 시험 18 + 캡처 진입 11
crates/tauri-plugin-screen-ink  명령·이벤트·트레이·팝오버/설정/섬 창 — install(builder) 한 줄로 붙임
packages/ink-client          InkClient 계약(Tauri·가짜) — UI가 만나는 유일한 문
packages/ink-ui              디자인 시스템(토큰·아이콘·부품)·화면(S01~S06·S16·S17)·미리보기
apps/screen-ink              단독 앱 셸
```

| 할 일 | 명령 |
|---|---|
| 의존성 | `npm install` |
| 시험 | `cargo test --workspace --exclude screen-ink --exclude tauri-plugin-screen-ink` · `npm test` · `npm run check` |
| 화면 미리보기(가짜 클라이언트) | `npm run ui:dev` → http://localhost:5180/?screen=tray · settings · island · board · first · cheat |
| 앱 실행 | `npm run app:dev` (트레이 상주, 첫 실행은 설정 창 + 안내 시트) |
| 단축키 자료 다시 만들기 | `npm run shortcuts:data` (입력표 → `docs/shortcuts/catalog-v1.json`) |
| 진입 시간(M1) 재기 | 앱을 띄운 채 `powershell -ExecutionPolicy Bypass -File bench\m1-overlay.ps1 -Count 100 -Save` |

- C 드라이브가 좁으면 Rust 빌드 폴더를 옮긴다: `$env:CARGO_TARGET_DIR='D:\dev-cache\screen-ink\target'`
- 학교망에서 crates.io 인증서 폐기 확인이 실패하면 그 창에서만 `$env:CARGO_HTTP_CHECK_REVOKE='false'`
- 개발 빌드는 로그인 자동 실행을 건드리지 않는다(릴리스 빌드만 `general.autostart`를 따름).
- 맥은 다른 환경에서 시험·조정한다(현재 맥 어댑터는 빈 탐지기).

## 후속(1차에서 뺀 것)

캡처 핀 · 캡처 주석 편집 · 스크롤 캡처 · OCR·QR · 라이브 판서(클릭 통과) · 판서 보존·스크롤 따라가기 · 타이머·교실 위젯 · 명령 팔레트 · 시스템 소리 녹음 · GIF · MS Store·공증 배포.

## 문서

- [docs/PRD.md](docs/PRD.md) — **1차 프로토타입 PRD v2.0**: 요구사항 ID·합격 기준·측정 목표·기술 근거·벤치마크 대조표 + 부속 문서 [docs/prd/](docs/prd/)(기능 목록 · 스마트 로직 · 구조 · **최종 화면 명세** · CRUD · 원격 · **마스터플랜** · **스마트 캡처 v2** · 디자인 방향 · **디자인 시스템**)
- [docs/shortcuts/README.md](docs/shortcuts/README.md) — **단축키 확정본 v1**(전역 16 · 모드 안 43) + 기계용 `defaults-v1.json`
- [docs/mockups/](docs/mockups/screen-ink-mockups.html) — **조작 가능한 화면 시안**(Tidy 디자인 15장, S02 = 단축키 59칸 편집) · `render.ps1`(이미지) · `qa.ps1`(자동 조작 시험 56개)
- [docs/mockups/v3/](docs/mockups/v3/png/contact.png) — **최종 시안 18장**(그래파이트 + 다크 아일랜드: 아이콘·토큰·트레이·설정 4·첫 실행·F1·F2·F3·판서·확대·칠판·창 고정·단축키 한 장·섬 12상태)
- [docs/prd/09-design-directions.md](docs/prd/09-design-directions.md) — 디자인 방향 고르기 기록(시안 5방향 [v2](docs/mockups/v2/png/contact.png) → E + B 확정)
- [docs/shortcuts/shortcut-picker.html](docs/shortcuts/shortcut-picker.html) — **단축키 입력표**(전역 16칸·모드 안 43칸 — 창 항상 위 고정 포함, Windows·macOS·Snipaste·아이캔 줌잇·ZoomIt·PowerToys·앱 단축키 겹침 바로 확인). 게시본: claude.ai 아티팩트(저장값을 Claude가 읽어 PRD에 반영)
- [docs/benchmark/](docs/benchmark/README.md) — 벤치마킹 방법(클린룸·지표·시나리오 카드)과 대상별 분석: [Snipaste 2.11.3](docs/benchmark/snipaste-2.11.3.md) · [아이캔 줌잇 0.7.2.2](docs/benchmark/icanzoomit-0.7.2.2.md) · [ZoomIt 0.94.1](docs/benchmark/zoomit-powertoys-0.94.1.md)
- [bench/](bench/) — 측정 스크립트(`snipaste-log-stats.ps1` · `sample-process.ps1` · `uia-probe.ps1` 앱별 UI 요소 노출)와 결과
- [docs/WINDOWS-SETUP.md](docs/WINDOWS-SETUP.md) — Windows PC 개발 환경 준비
- [docs/아이디어-고도화-1차.md](docs/아이디어-고도화-1차.md) — 벤치마크·기회·기능 아이디어·플랫폼·배포·13절 프로토타입 목표 v0.1(구글 독스 사본)
- [docs/research/](docs/research/) — 조사 원문(Antigravity 딥서칭 · Codex 아이디어 · Claude 사실 확인 · 오픈소스 조사)
