# 2차 고도화 화면 스냅숏·성능 QA 묶음 (2026-11-27)

- **대상 요구사항**: `R2-QA-1` (미리보기 화면 스냅숏 시험) · `R2-PERF-1` (2차 성능 예산 측정)
- **실행 스크립트**: [scripts/qa-snapshots.mjs](../../../scripts/qa-snapshots.mjs)
- **브라우저·환경**: `chromium-1248` (`@playwright/test@1.64.1`, 기존 맥 캐시 사용 · 새 브라우저 다운로드 없음), 뷰포트 `1280×820` (`deviceScaleFactor: 1`), `FakeInkClient` 기반 `@screen-ink/ui` Vite 미리보기
- **사람 검수용 PNG 저장 위치**: `output/qa/2026-11-27/` (`.gitignore` 제외 경로 · 총 168장 · 커밋하지 않음)
- **원본 측정 JSON**: `output/qa/2026-11-27/results.json` (커밋하지 않음)

## 보고서 목록

1. **[snapshots.md](snapshots.md)** — 168장 스냅숏 자동 검사(넘침·잘림·경계 이탈·겹침·OS 표기 섞임) 결과, 눈 검수 추가 발견, 단축키 PDF 모델 검증 결과, 발견된 화면 문제 목록
2. **[perf.md](perf.md)** — `R2-PERF-1` 성능 예산(글꼴 첫 표시 지연 · 색·굵기 패널 열기 · 글꼴 패널 첫 열기) 측정표 및 예산 대조·원인 분석
3. **[checklist-2.md](../../release/checklist-2.md)** — `R2-REL-1` 릴리스 점검표 2차판 (1차 7.5 통합)

## 한눈에 보는 요약

| 항목 | 결과 | 비고 |
|---|---|---|
| 촬영 스냅숏 수 | **168장** (20종 화면 × 8조합 + 다크 전용 2종 × 4조합) | 미발견 0 · 콘솔 오류 0 · 페이지 오류 0 |
| OS 표기 섞임 (`R2-OSFIT-3`) | **0건** (168장 + PDF 4조합) | 맥 `Ctrl/Alt/Win/.exe/작업 표시줄/탐색기` 0, Win `⌘/⌥/⌃/메뉴 막대/Finder/시스템 설정` 0 |
| 경계 이탈 · 뷰포트 잘림 · 겹침 | **0건** (자동 검사 기준) | 단, `island-gallery` 미리보기 3열 그리드 폭 부족으로 결과 섬 2장이 눈 검수에서 겹침 확인 |
| 가로 넘침 (`scrollWidth > clientWidth`) | **12장** (2개 원인) | `first-step3` 원시 i18n 키(8장), `tray` 맥 `[전체]` 타일 +3 px(4장) |
| 잘림(말줄임 `ellipsis`) | **20장** (2개 원인) | `settings-shortcuts`/`first-step3` 원시 i18n 키(16장), `island-gallery` 결과 섬 보조 글(4장) |
| 눈 검수 추가 발견 | **2건** | 영어 설정 단축키 충돌 배너 한국어 잔존 · `overlayPreviewClient`의 `systemFonts` 미위임 |
| `R2-PERF-1` 성능 예산 (Chromium) | **3/3 통과** (단서 1건) | 글꼴 17.0 ms(≤30) · 색·굵기 7.8 ms(≤16) · 글꼴 패널 8.0 ms(≤120, 시스템 글꼴 미로드 단서) |
| PDF 미리보기 화면 | **미구현(P1 `R2-PDF-7`)** | 대체로 `buildShortcutSheetPdf` 4조합(`ko/en × mac/win`) 생성·OS 표기 0건 검증 |

## 사람 검수용 PNG 파일 명명 규칙

```
output/qa/2026-11-27/<화면id>__<platform>_<lang>_<theme>.png
```

- `<platform>`: `macos` · `windows`
- `<lang>`: `ko` · `en`
- `<theme>`: `light` · `dark` (단, `cheat-sheet`·`island-gallery`는 앱 자체가 다크 고정이라 `dark`만 생성)

## 재실행 명령

```bash
/Users/bodyking/.local/bin/resource-guard run --profile test --wait-seconds 900 -- \
  bash -c 'timeout 900 node scripts/qa-snapshots.mjs'
```
