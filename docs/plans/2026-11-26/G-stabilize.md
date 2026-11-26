# G. 단계 7 안정화 · 벤치마크 · 설치본 구현 시안 (`7.1`~`7.7`)

대상 항목(`progress.json`): `7.1` 벤치 M1~M10 학교 PC·혼합 배율 · `7.2` 시나리오 카드 S1~S15 · `7.3` 인식 회귀·CI 기준선 고정 · `7.4` 200회 진입·종료(메모리·핸들) · `7.5` 릴리스 체크리스트(`R-NET-2`·`R-LOG-1` 등) · `7.6` NSIS 설치본·포터블 ZIP · `7.7` 디자인 관문(S01~S17 = v3).

---

## 1. 목표 · 범위 (요구사항 ID · 관문 G7)

| 요구사항 / 관문 | 요약 |
|---|---|
| `M1`~`M10` (`PRD §5`) | 캡처 진입(p50≤80/p95≤150 ms), 시작(≤1.5 s), 대기 자원(≤150 MB·CPU≈0%), 판서 진입(≤150 ms), 펜 지연(p95≤50 ms), 실시간 확대(1080p≥30 fps), 캡처 정확도(0 px), 인식 정확도(Top-1≥90%/Top-3≥98%), 안정성(200회 메모리+5%·핸들 0), Snipaste 대비 |
| `R-NET-2` | Rust 의존성 HTTP 클라이언트 0건(`cargo tree`), 웹뷰 CSP `connect-src`, PE import에 `winhttp`·`wininet`·`ws2_32` 없음 |
| `R-LOG-1` | 7일 순환 로그에 창 제목·창 안 글자·파일 이름·사용자 이름 0건 |
| `R-SYS-5`·`R-SET-1` | 표준 사용자 계정 설치/실행, NSIS 설치본 + `screen-ink.portable` 포함 포터블 ZIP |
| `R-UI-2·4`·`09 §6` | S01~S17 v3 시안 대조, AI slop 점검표 10개, 대비 4.5:1, `forced-colors`, `prefers-reduced-motion`, 라이트/다크 전환 |
| **관문 G7** | `07-plan.md §6` 산출물 체크리스트 11개 항목 전부 통과 |

---

## 2. 정확한 형식

### 2.1 자동 검증 스크립트 · 산출물 위치

| 항목 | 파일 / 스크립트 위치 | 입력 · 출력 형식 |
|---|---|---|
| `7.1` 벤치 묶음 | `bench/run-all-metrics.ps1` (신규) → `bench/results/2026-11-xx-m1-m10-school.md` | `m1-overlay.ps1`, `sample-process.ps1`, 시작 시간 20회, `mode enter draw`, `pen latency`, PresentMon M6, 시험 패턴 M7(픽셀 diff 0) 집계표 |
| `7.2` 시나리오 카드 | `bench/results/2026-11-xx-scenarios-s1-s15.md` | S1~S15 표(`screen-ink` vs `Snipaste` vs `아이캔 줌잇` vs `ZoomIt`, 판정 `○/△/×` + 비고) |
| `7.3` 인식 회귀 | `bench/recog/cases/baseline.json` + `.github/workflows/ci.yml` | 전체 RB 사례 재생 결과를 기준선으로 고정, 하락 시 CI 실패 |
| `7.4` 200회 누수 시험 | `bench/soak-200.ps1` (신규) → `bench/results/2026-11-xx-m9-soak200.csv` | F1 진입/Esc 100회 + Alt+1 진입/Esc 100회 반복 전후 `WorkingSet64`, `HandleCount`, `GDI Objects`, `USER Objects` 기록 |
| `7.5` 릴리스 점검 | `scripts/release-check.ps1` (신규) → `bench/results/2026-11-xx-release-check.md` | 5종 자동 검사(아래 2.2) + 수동 점검(표준 사용자·백신 오탐·UPX 미사용) |
| `7.6` 패키징 | `scripts/build-portable.ps1` (신규) + `tauri.conf.json` NSIS 빌드 | 출력: `dist-release/screen-ink_<ver>_x64-setup.exe` (NSIS `currentUser`) + `dist-release/screen-ink_<ver>_x64-portable.zip` (`screen-ink.exe` + `screen-ink.portable` 빈 표시 파일 + `LICENSE`) |
| `7.7` 디자인 관문 | `scripts/design-gate.mjs` (신규) + `bench/results/2026-11-xx-design-gate.md` | ① 하드코딩 색 린트(토큰 외 `#hex`/`rgb` 0건) ② 대비 4.5:1 자동 검사(라이트/다크 토큰 쌍) ③ S01~S17 실기 캡처 ↔ `docs/mockups/v3/png/F01~F18` 나란히 대조표 ④ AI slop 10개 점검 |

### 2.2 `scripts/release-check.ps1` 자동 검사 항목 (`7.5`)

```powershell
# 1) HTTP 클라이언트 크레이트 금지 (R-NET-2)
cargo tree --workspace | Select-String -Pattern '\b(reqwest|ureq|hyper|surf|isahc|curl|tauri-plugin-http|tauri-plugin-updater)\b'
# -> 0건이어야 통과

# 2) PE Import 검사 (R-NET-2) — llvm-objdump / dumpbin / python pefile 또는 rea inspect-pe-imports
# 대상: target/release/screen-ink.exe
# 금지 DLL: winhttp.dll, wininet.dll, ws2_32.dll (WebView2Loader 자체 통신 옵션 별도 확인)

# 3) CSP 검사 (apps/screen-ink/src-tauri/tauri.conf.json)
# connect-src에 외부 https/http 도메인 없음(ipc:/ink:/localhost만 허용)

# 4) 로그 개인정보 검사 (R-LOG-1)
# 테스트 중 띄운 특정 창 제목 토큰("SCREEN_INK_CANARY_TITLE_99")을 로그 전체에서 검색 -> 0건

# 5) UPX 패킹 금지 검사
# PE 섹션 이름에 UPX0/UPX1 없음 확인
```

> **[확인 필요]** `R-NET-2`에 명시된 WebView2 런타임 자체 통신(텔레메트리/SmartScreen 등)을 끄는 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` 실행 옵션(`--disable-background-networking` 등) 지원 여부 및 부작용을 `7.5`에서 실측 확인한다.

### 2.3 포터블 ZIP (`7.6`) 구조

```
screen-ink_<ver>_x64-portable.zip
├─ screen-ink.exe
├─ screen-ink.portable     # 0바이트 표시 파일 (R-SET-1, ink-store::resolve_dir가 감지)
└─ LICENSE                 # MIT
```

- 실행 시 같은 폴더에 `data/settings.json`과 `data/logs/`가 생겨야 하며, `%APPDATA%\screen-ink`에는 아무 파일도 쓰지 않음을 자동 시험으로 확인한다.

---

## 3. 알고리즘 · 결정 (대안과 고른 이유)

- **200회 누수 시험(M9) 기준**: 작업 집합(WebView2 자식 프로세스 합산) 증가 `+5%` 미만, Win32 핸들(특히 GDI `HBITMAP`/`HDC`와 `USER` `HWND`) 증가 `0`(허용 오차 `±5` — OS 내부 캐시 변동 감안하되 단조 증가 기울기 0).
- **M7 캡처 정확도 0 px 검증**: 100% + 150% 혼합 배율 모니터에 1px 체커보드와 꼭짓점 색 마커(좌상단 `#FF0000`, 우하단 `#0000FF`) 시험 HTML을 띄우고, 알려진 물리 사각형 좌표를 캡처해 결과 PNG의 네 꼭짓점 색과 폭·높이가 정확히 일치하는지 자동 비교한다.
- **S1~S15 비교 공정성**: `Snipaste 2.11.3`·`아이캔 줌잇 0.7.2.2`·`ZoomIt 0.94.1` 모두 기본 설정으로 같은 학교 PC에서 동일 시나리오를 수행하고, 불리한 결과도 숨기지 않고 `△/×`로 기록한다.

---

## 4. 시험 · 합격 기준 (관문 G7)

| 항목 | 합격 기준 |
|---|---|
| `7.1` M1~M10 | `PRD §5` 목표표 전 항목 통과 (학교 PC + 100%+150% 혼합 배율) |
| `7.2` S1~S15 | 1차 범위 시나리오 전부 `○` |
| `7.3` 인식 회귀 | `cargo test` + `recog-replay` CI 100% 통과, `baseline.json` 대비 하락 0 |
| `7.4` M9 안정성 | 200회 반복 후 메모리 증가 `< +5%`, GDI/USER/Kernel 핸들 단조 증가 `0` |
| `7.5` 릴리스 체크 | `release-check.ps1` 5항목 0건 + Windows Defender 오탐 0 + 표준 사용자 계정 정상 실행 |
| `7.6` 배포물 | NSIS 설치 → 단축키 동작 → 제거 깨끗함; 포터블 ZIP 다른 폴더로 이동 시 설정 유지 |
| `7.7` 디자인 관문 | 하드코딩 색 0, 대비 4.5:1 전 쌍 통과, v3 시안 차이 목록 0(의도 차이 제외), 고대비·줄임 모드 정상 |

---

## 5. 위험 · 대응

| 위험 | 발생 신호 | 대응 |
|---|---|---|
| **`ws2_32.dll` 간접 import 검출** (Rust std/tokio 기본 소켓 심볼) | `release-check.ps1` PE 검사 실패 | `tokio` features에서 `net` 제거(`rt`, `sync`, `time`, `macros`만 사용), `Cargo.toml` 의존성 정리 후 재검사 |
| **200회 반복 시 WebView2 메모리 서서히 증가** | M9 `+5%` 초과 | 오버레이 `clearOverlay()`에서 `offscreenCanvas.width = 0`·`img.removeAttribute('src')` 확인 + `FrameStore`/`ThumbStore` 해제 누수 점검 |
| **무서명 NSIS / 포터블 ZIP 백신 오탐** | Windows Defender 격리 | UPX 미사용 확인 + 실제 배포는 묶음 **H**의 Microsoft Store MSIX(Microsoft 서명) / SignPath 서명 MSIX로 전환 |
| **혼합 배율(100%+150%) 물리 모니터 부재** | 학교 PC가 1080p×2 동배율 | 한쪽 모니터 Windows 디스플레이 배율만 임시로 150%로 바꿔 S10·M7·RB-12 측정 후 원복 |

---

## 6. 작업일 · 난이도 · 먼저 끝나야 할 것

| 항목 | 작업일 | 난이도 | 선행 |
|---|---:|---|---|
| `7.1` 벤치 M1~M10 학교 PC·혼합 배율 | 0.36일 | 중 | **A·B·C·D·E** |
| `7.2` 시나리오 카드 S1~S15 | 0.36일 | 중 | **A·B·C·D·E·(F)** |
| `7.3` 인식 회귀·CI 기준선 고정 | 0.36일 | 하 | **C** |
| `7.4` 200회 진입·종료(M9) | 0.35일 | 하 | **A·B·D·E** |
| `7.5` 릴리스 체크리스트 | 0.35일 | 중 | 전체 기능 동결 후 |
| `7.6` NSIS 설치본·포터블 ZIP | 0.36일 | 중 | `7.5` |
| `7.7` 디자인 관문 (S01~S17 = v3) | 0.36일 | 중 | **A·B·D·E·F·J** |
| **묶음 합계** | **2.50일** | **중** | **A·B·C·D·E·(F)·J 완료** |
