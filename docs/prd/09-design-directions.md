# 09. 화면 디자인 v2 — 시안 5방향과 고도화 계획

2026-11-25 · PRD v3.0 부속 · 상태: **선택 완료 — E 그래파이트 + B 아일랜드 다크**(2026-11-25). 결과는 [10-design-system](10-design-system.md) · [04-screens](04-screens.md), 최종 시안은 [mockups/v3](../mockups/v3/png/contact.png).

- 시안 캔버스(비공개, 소유자만 열림): https://claude.ai/artifact/1WMRkEX8nkA8tUkUwjhWFm — 맨 위 "한눈에 보기" 1장 + 방향마다 4장(트레이 팝오버 · 설정 단축키 · F1 캡처 화면 · 요소 시트)
- 원본: [docs/mockups/v2/](../mockups/v2/) (`*.dc.html` · `canvas.json`) · 이미지: [docs/mockups/v2/png/](../mockups/v2/png/) · 모아보기: [contact.png](../mockups/v2/png/contact.png)
- 조사: Antigravity(네온 오케스트레이션) 디자인 조사 답장(2026-11-25) — 방향별 토큰·함정, 메뉴 막대 앱 사례, 라이브러리, 에셋 규격, AI slop 점검표. 아래 표에 합쳐 적었다.

## 0. 왜 다시 하나 — 기존 시안의 문제

| 문제 | 기존 시안(v0.3) | v2 원칙 |
|---|---|---|
| 장식 과잉 | 점무늬 말풍선, 점선 테두리, 2px 입체 아랫선 단추 | 1px 머리카락 선 + 옅은 그림자만 |
| 색 분산 | 한 화면에 색 방울 아이콘 7색 | 무채색 90% + 강조 1색(D만 모드 색을 10% 바탕으로) |
| 상자 겹침 | 탭 + 알약 머리 + 카드 + 알약 행 | 묶음 상자 1단계, 안쪽은 선으로만 나눔 |
| 글 과밀 | 긴 구어체 설명, 줄바꿈 | 라벨 12자 이내 한 줄, 설명은 빼거나 24자 한 줄 |
| 확인 단추 | 설정 창 아래 [취소]/[적용]/[완료] + 변경 개수 | 바로 반영(애플 방식), 위험한 동작만 확인 시트 |

## 1. 다섯 방향이 함께 지키는 것

1. **한 화면에 초점 하나.** 팝오버는 항목 10개 이내, 설정 화면은 묶음 2~4개, 캡처 화면의 안내는 배지 1개 + 키 안내 1줄.
2. **유목화**: 모든 목록은 인덱스 제목(11~12px, 보조 색) 아래 묶는다. 같은 묶음 이름을 팝오버·설정·안내 한 장에서 똑같이 쓴다 — **캡처·녹화 / 화면 도구 / 빠른 동작**.
3. **라벨은 한 줄**(`white-space: nowrap`, 본문은 `word-break: keep-all`), 키는 오른쪽 끝에 정렬(`tabular-nums`).
4. **상태는 점 하나로**: 준비됨(초록·주황), 녹화 중(빨강), 겹침(주황). 겹침은 설정 맨 위 한 줄 알림 + 해당 줄 점 → "Snipaste 끄는 법"(R-KEY-5).
5. **아이콘**: 1.5~2px 선, 끝 둥글게. 이모지 금지. Apple SF Symbols는 Apple 기기 전용 약관이라 쓰지 않는다.
6. **화면 위 안내는 화면을 가리지 않는다**: 1줄 알약, 결과 말풍선은 1.2초 뒤 사라짐(마우스를 올리면 머묾).

## 2. 공통 정보 구조

**트레이 팝오버**(폭 284~304px, 항목 9개)

| 구역 | 항목 | 키 |
|---|---|---|
| 머리 | 앱 아이콘 · screen-ink · 상태 점(준비됨 / 꺼짐 / 녹화 중) | — |
| 캡처·녹화 | 범위 캡처 · 창 캡처 · 화면 녹화 | F1 · F2 · F3 |
| 화면 도구 | 판서 · 실시간 확대 · 칠판 · 창 항상 위(Windows만) | Alt+1 · Alt+2 · Alt+3 · Alt+Shift+D |
| 빠른 동작 | 직전 판서 열기 · 저장 폴더 열기 | Alt+Shift+1 · — |
| 바닥 | 단축키 켜기 스위치 · 설정… · 끝내기 | `` Alt+` `` |

캡처 기록 지우기·팁 문구·버전은 팝오버에 두지 않고 설정으로 옮긴다.

**설정 창**(900×600, 사이드바 ~200px): 단축키 · 캡처·녹화 · 판서·확대 · 일반 · (아래) 정보. 단축키 화면은 [어디서나 16 | 화면 안 43]으로 나누고, 묶음 = 캡처 7 · 화면 도구 4 · 빠른 동작 2.

**F1 캡처 화면**: 강조 테두리 · 경로와 크기 배지 1개(창 › 표 › 2행 · 718 × 40) · 확대경(색·좌표) · 키 안내 1줄(휠·Tab·Enter·Esc).

## 3. 다섯 방향

| | A 클리어 | B 아일랜드 | C 문구점 | D 젤리 | E 그래파이트 |
|---|---|---|---|---|---|
| 한 줄 | macOS 설정의 정석 | 화면 위 검은 알약 하나가 늘고 줄어듦 | 따뜻한 종이와 색연필 | 모드마다 파스텔, 말랑한 면 | 모노톤 도구에 주황 한 점 |
| 바탕 · 면 · 글자 | #F5F5F7 · #FFFFFF · #1D1D1F | #111214 · #1C1D21 · #F7F8FA | #FAF8F4 · #FFFFFF · #1F1E1C | #F6F5FB · #FFFFFF · #24223A | #EFEFEC · #FFFFFF · #121212 |
| 보조 · 구분선 | #6E6E73 · #E5E5EA | #8E939E · #2C2E35 | #6F6B64 · #E9E4DA | #6B6789 · #ECEAF5 | #6B6B68 · #DDDDD8 |
| 강조 | 파랑 #0A7CFF | 노랑 #FFD60A | 잉크 주황 #D97706 | 라벤더 #7C5CFF | 주황 #FF9F0A |
| 모서리 | 창 12 · 묶음 10 · 행 7 | 창 18 · 팝오버 22 · 알약 999 | 창 14 · 묶음 12 · 키 5 | 창 26 · 묶음 18 · 타일 16 | 창 12 · 묶음 8 · 키 4 |
| 재질 | 흐림 + 0.5px 선 | 검은 유리 + 1px 빛 테두리 | 불투명 종이 + 1px 연필선 | 무광 + 라벤더 그림자 | 불투명 + 1px 선 |
| 선택 표시 | 파란 행 | 흰 타일(반전) | **형광펜 밑줄** | 라벤더 고리 | 검정 행(반전) + 주황 키 |
| 아이콘 | Lucide 1.7px | Lucide 1.8px | 둥근 끝 2px(Phosphor) | 파스텔 칩 안 2px(Phosphor) | Lucide 1.5px |
| 글꼴(앱) / 시안 대용 | Pretendard / Noto Sans KR | Wanted Sans 또는 Pretendard / Noto Sans KR | SUIT + 고운돋움 제목 / 고운돋움 | LINE Seed KR 또는 Pretendard / Noto Sans KR | IBM Plex Sans KR + Plex Mono / 같음 |
| 움직임 | 180 ms 감속, 튐 없음 | 알약 폭·높이 스프링 모핑 | 160 ms 감속 | 스프링 + 눌림 0.97 | 120 ms 즉시 |
| 귀여움 | 앱 아이콘·트레이 점 | 알약이 말하듯 움직임 | 형광펜·필통 펜 색 | 가장 강함(모드 타일) | 주황 점 하나 |
| 함정(피할 것) | 불투명 바탕 위 가짜 유리 | 알약이 커져 수업 화면을 가림 · 단추 과밀 | 스티커·점선 과다 · 누런 대비 부족 | 고채도 무지개 · 유아용 느낌 | 순검정 + 형광 빛 · 개발자 대시보드화 |

시안의 글꼴은 캔버스가 Google Fonts만 쓸 수 있어 대용이다. 앱에는 woff2로 넣는다(모두 SIL OFL 1.1).

## 4. 고를 때 볼 점

| 질문 | 유리한 방향 |
|---|---|
| 교실 TV·프로젝터에서 화면 위 안내가 잘 보이나 | B(검은 알약) · E(짙은 막대) |
| 하루 종일 켜 둬도 피곤하지 않나 | A · E |
| "아기자기"가 가장 잘 느껴지나 | D · C |
| Windows 11에서도 어색하지 않나 | E · B(A는 맥 색이 강함) |
| 판서 색(R·Y·G·O·B·L)과 화면 색이 부딪치지 않나 | A · B · E(무채색 바탕) |

섞어 쓸 수도 있다: 예) 설정·팝오버는 A, 화면 위 안내는 B의 알약.

## 5. 고른 뒤 고도화 계획

### 5.1 순서

| # | 할 일 | 끝 기준 |
|---|---|---|
| 1 | 고른 방향으로 S01~S15 전면 재작성 + 상태별 그림(기본·마우스 올림·누름·비활성·오류) + 다크 모드 | 시안 자동 조작 시험 다시 통과, §6 점검표 10개 통과 |
| 2 | 실제 글꼴(Pretendard 등)을 받아 시안에 적용 — **파일 내려받기는 사용자 승인 뒤** | 대용 글꼴과 줄 길이 차이 확인 |
| 3 | 디자인 토큰 한 벌: `packages/ink-ui/tokens.css`(라이트·다크) → Tailwind 4 `@theme` 연결, Tidy `--st-*` 이름과 대응표 | 시안과 앱이 같은 변수 사용 |
| 4 | 앱 아이콘 1024px 마스터(아래 프롬프트로 초안 생성 → 벡터로 다듬기) → `tauri icon` | Windows ico 16~256 · macOS icns 생성 |
| 5 | 트레이 아이콘: Windows 16·20·24·32px 라이트/다크 + 녹화·꺼짐 변형, macOS 템플릿 16pt(@2x) `iconAsTemplate` | 작업 표시줄 라이트·다크에서 또렷함 |
| 6 | 부품 목록(팝오버 행·묶음·키캡·스위치·나눔 단추·알약·말풍선·확인 시트)을 `ink-ui`에 Svelte 5로 | 미리보기(FakeInkClient)에서 시안과 같음 |

### 5.2 라이브러리·패키지 (Tauri 2 + Svelte 5)

| 역할 | 후보 | 라이선스 | 메모 |
|---|---|---|---|
| 창 재질(Win11 Mica·Acrylic / macOS 흐림) | Tauri 2 `WindowEffectsConfig` · `window-vibrancy` | Apache-2.0/MIT | A·B 방향에 필요. 학교 PC가 Windows 10이면 단색으로 물러남 [확인 필요] |
| 트레이 기준 팝오버 위치 | `tauri-plugin-positioner`(`tray-icon` 기능) | Apache-2.0/MIT | 작업 표시줄 위치·모니터 배율 자동 |
| 접근성 있는 기본 부품 | `bits-ui` | MIT | 스위치·나눔 단추·대화 상자·메뉴. Svelte 5 버전 [설치 때 확인] |
| 선 아이콘 | `@lucide/svelte` | ISC | A·B·E |
| 두께 여러 단계 아이콘 | `phosphor-svelte` | MIT | C·D [버전 확인] |
| 움직임 | Svelte 5 내장 `svelte/motion`(`Spring`·`Tween`) | MIT | 추가 묶음 0. `prefers-reduced-motion`이면 끔 |
| 한글 글꼴 | Pretendard · Wanted Sans · SUIT · LINE Seed KR · IBM Plex Sans KR | SIL OFL 1.1 | 쓰는 글자만 남긴 woff2로 앱에 넣음(오프라인·네트워크 0) |

### 5.3 에셋

| 에셋 | 규격 | 만드는 법 |
|---|---|---|
| 앱 아이콘 마스터 | 1024×1024 PNG(투명), 안전 영역 824px | 이미지 생성 초안 → 벡터 다듬기 → `pnpm tauri icon` |
| Windows 아이콘 | ico 16·24·32·48·64·256 | `tauri icon` 자동 |
| macOS 아이콘 | icns 16~1024(@1x/@2x) | `tauri icon` 자동 |
| 트레이 아이콘 | Win 16·20·24·32 PNG(라이트·다크·녹화·꺼짐), mac 16pt 템플릿 | 벡터 → 정수 화소 맞춤 PNG |
| 빈 화면 그림 | 240×160 SVG 2장(첫 실행 · 기록 없음) | 단색 선 그림, 화면당 최대 1개 |

방향별 앱 아이콘 초안 생성 프롬프트(Antigravity 제안, 영어 그대로 쓴다):

- **A**: `Minimalist macOS Big Sur style app icon on a clean squircle canvas, translucent frosted glass frame holding a crisp cobalt blue stylus nib drawing a luminous curved ink stroke, soft studio top-left lighting, subtle inner rim light, ultra-clean vector 3D, centered, transparent background, no text`
- **B**: `Sleek macOS squircle app icon, matte obsidian graphite tile featuring a glossy black pill silhouette with a precise white pen tip and a tiny warm yellow status dot, subtle specular highlight, modern hardware finish, centered, no text`
- **C**: `Charming minimalist macOS squircle app icon, warm ivory paper tile with a tactile amber-orange pen nib sketching a clean bracket frame, soft matte clay shading, gentle natural shadow, refined stationery vibe, centered, no text`
- **D**: `Cute refined macOS squircle app icon, soft cloud-white squircle with a plump pastel lavender frosted highlighter resting diagonally over a rounded crop corner, tactile soft 3D clay-glass finish, clean composition, no text`
- **E**: `Precision pro-tool macOS squircle app icon, brushed dark graphite surface with an engraved geometric viewfinder reticle and a single sharp amber ink drop accent, crisp micro-bevel lighting, ultra-minimalist industrial design, centered, no text`

### 5.4 움직임

| 장면 | 움직임 |
|---|---|
| 팝오버 열기·닫기 | 투명도 0→1 + 크기 0.98→1, 160~180 ms 감속(E는 120 ms) |
| 알약(B) 늘고 줄기 | 폭·높이 스프링(단단함 0.22 · 감쇠 0.78) |
| 결과 말풍선 | 아래에서 6px 올라오며 나타남, 1.2초 뒤 사라짐, 마우스를 올리면 머묾 |
| 누름 | 크기 0.97(D) / 없음(A·E) |
| 줄임 모드 | `prefers-reduced-motion`이면 투명도만 |

## 6. AI slop 점검표 (시안·구현 검토 관문)

| # | 금지 | 기준 |
|---|---|---|
| 1 | 한 화면 원색 아이콘 5~7색 | 무채색 90% + 강조 1색 |
| 2 | 점무늬·점선·입체 아랫선·형광 빛 | 1px 선 + 옅은 그림자 |
| 3 | 카드 안 카드, 행마다 알약 상자 | 묶음 상자 1단계 + 안쪽 선 |
| 4 | 기본 글꼴(맑은 고딕)·굵기 남발 | 글꼴 1벌, 굵기 2~3단계 |
| 5 | 라벨·단추가 두 줄로 꺾임 | 라벨 12자 한 줄 |
| 6 | 친근한 척 긴 설명·이모지·괄호 부연 | 명사·동사 라벨, 필요한 1줄만 |
| 7 | 설정 창 아래 [취소][적용][완료] | 바로 반영, 위험 동작만 확인 |
| 8 | 팝오버 12개 이상·관리 기능 섞임 | 3~4묶음, 10개 이내 |
| 9 | 캡처·판서 중 화면을 가리는 큰 막대 | 1줄 알약·배지, 1.2초 소멸 |
| 10 | 밝은 화면을 뒤집기만 한 다크 모드 | 다크 면 계층(바탕 < 면 < 띄운 면) 따로 |

그라데이션으로 바탕을 덮는 장식, 왼쪽 굵은 띠 카드, 가짜 숫자·지어낸 문구도 쓰지 않는다.

## 7. 열린 결정

1. **다섯 방향 중 하나**(또는 섞기) — 사용자 선택.
2. **설정 바로 반영**으로 바꿀지 — 바꾸면 PRD R-SET-0·R-KEY-3의 [적용] 트랜잭션은 "키를 입력한 순간 등록 시도 → 실패하면 그 줄에서 바로 되돌림 + 이유"로 바뀐다(05 §1·§2 수정). 다섯 시안은 모두 바로 반영으로 그렸다.
3. 다크 모드를 1차 기본 지원으로 할지(B는 다크가 기본).
