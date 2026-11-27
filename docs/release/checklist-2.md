# 슉쇽(ShookShok) 릴리스 점검표 2차판 (`R2-REL-1`)

- **대상 판**: 2차 고도화 (`0.2.0` 이후 macOS · Windows 공통 릴리스 기준)
- **통합 원본**:
  - 1차 릴리스 체크리스트 및 완료 정의: [docs/prd/07-plan.md](../prd/07-plan.md) §4(7.5)·§6, [docs/plans/2026-11-26/G-stabilize.md](../plans/2026-11-26/G-stabilize.md) §2.2
  - macOS 서명·권한·업데이트: [docs/handoff/mac-self-sign.md](../handoff/mac-self-sign.md), [docs/plans/2026-11-26/I-mac.md](../plans/2026-11-26/I-mac.md), `scripts/release-mac.sh`, `crates/ink-platform-macos/src/updater.rs`
  - Windows MSIX·배포: [docs/plans/2026-11-26/H-distribution.md](../plans/2026-11-26/H-distribution.md), [docs/store/msix-packaging.md](../store/msix-packaging.md)
  - 2차 고도화 PRD: [docs/plans/2026-11-27/PRD-2차-고도화.md](../plans/2026-11-27/PRD-2차-고도화.md) §4·§6·§7·§8
- **원칙**: 자동 검사 항목은 명령 출력으로 증거를 남기고, 사람 실기·게시 항목은 확인자와 날짜를 맨 아래 서명 표에 기록한다. `[ ]`가 하나라도 남으면 릴리스를 공개하지 않는다(초안 유지).

---

## 1. 공통 코드·품질·버전 관문 (1차 + 2차 통합)

### 1.1 버전·메타데이터 일치
- [ ] `apps/screen-ink/src-tauri/tauri.conf.json`의 `version`을 릴리스 판(예: `0.2.0`)으로 올렸는지 확인 (macOS 자동 업데이터가 `package_info().version`과 `latest-mac.json`을 비교한다).
- [ ] `Cargo.toml`의 `workspace.package.version`과 Windows MSIX 4자리 버전(`X.Y.Z.0`)이 같은 판을 가리키는지 확인.
- [ ] `git status`에 미커밋 변경·임시 파일·비밀값이 없고, 릴리스 기준 태그(`vX.Y.Z`) 커밋이 원격에 푸시되었는지 확인.

### 1.2 자동 시험·정적 검사
- [ ] Rust 전체 시험·린트 통과:
  ```bash
  cargo test --workspace
  cargo clippy --workspace --all-targets -- -D warnings
  ```
- [ ] 프런트엔드 단위 시험·타입 검사·카탈로그 정합성 통과:
  ```bash
  npm test
  npm run check
  npm run i18n:check
  npm run progress -- --check
  ```
- [ ] 인식 회귀 기준선(`bench/recog/cases/baseline.json`) 하락 0건 확인 (`recog-replay` / CI 통과).

### 1.3 2차 UI·다국어·OS 맞춤·접근성·성능 관문
- [ ] **OS 표기 섞임 0건 (`R2-OSFIT-3`)**: `packages/ink-ui/src/lib/i18n/osWords.test.js` 통과 + 미리보기 스냅숏 검사에서 맥 화면 `Ctrl/Alt/Win/.exe/작업 표시줄/탐색기` 0건, Windows 화면 `⌘/⌥/⌃/Cmd/Option/메뉴 막대/Finder/시스템 설정` 0건.
- [ ] **화면 스냅숏 넘침·잘림 0건 (`R2-QA-1`)**: `node scripts/qa-snapshots.mjs` 실행 후 [docs/qa/2026-11-27/snapshots.md](../qa/2026-11-27/snapshots.md)의 넘침·잘림·겹침 결함이 모두 해소되었는지 확인.
- [ ] **단축키 PDF 검증 (`R2-PDF-3`)**: `packages/ink-ui/src/lib/pdf/pdf.test.js` 통과 — `%PDF-` 헤더, A4 가로(영어 US 지역은 Letter), 5종 TTF 임베딩, 쪽 넘김, 바꾼 키 반영 확인.
- [ ] **접근성 (`R2-A11Y-1~3` · 1차 7.7)**: 글자 대비 ≥ 4.5:1 · 아이콘/테두리 ≥ 3:1, 모든 버튼 `aria-label`, 키보드만으로 주요 화면 조작, `prefers-reduced-motion`·`prefers-reduced-transparency`·Windows 고대비(`forced-colors`) 정상.
- [ ] **성능 예산 (`R2-PERF-1` + 1차 PRD §5)**:
  - 2차 예산([docs/qa/2026-11-27/perf.md](../qa/2026-11-27/perf.md)): 글꼴 첫 표시 지연 ≤ 30 ms (`font-display: block`), 색·굵기 패널 열기 ≤ 16 ms, 글꼴 패널 첫 열기 ≤ 120 ms.
  - 1차 예산: 캡처 진입 p50 ≤ 80 ms / p95 ≤ 150 ms, 시작 p50 ≤ 1.5 s, 대기 메모리 ≤ 150 MB, 200회 진입·종료 메모리 증가 < +5% · 핸들 증가 0.

---

## 2. macOS 서명 · 지정 요구 사항(DR) · 권한 유지 (`M-sign` · `R2-ONB`)

### 2.1 자체 서명 인증서 및 백업
- [ ] 빌드 맥 키체인에 고정 자체 서명 코드 서명 인증서(`Shookshok Self Sign`)가 유효한지 확인:
  ```bash
  security find-identity -p codesigning
  ```
- [ ] 인증서 개인 키(`.p12`, 암호 설정)가 **사람의 안전한 백업(Bitwarden `ops` 등)**에 보관되어 있는지 확인 (저장소·대화·로그·메신저 업로드 절대 금지; 분실 후 재발급하면 DR이 달라져 전 사용자의 권한이 풀린다).

### 2.2 서명 무결성 및 DR 동일성
- [ ] 고정 서명 빌드 수행:
  ```bash
  APPLE_SIGNING_IDENTITY="Shookshok Self Sign" sh scripts/mac-build-dmg.sh
  ```
- [ ] 엄격 서명 검증 통과:
  ```bash
  codesign --verify --deep --strict target/release/bundle/macos/Shookshok.app
  ```
- [ ] **지정 요구 사항(DR)이 직전 릴리스(`target/mac-dr.txt`)와 글자 하나까지 동일한지 확인**:
  ```bash
  codesign -dr - target/release/bundle/macos/Shookshok.app
  ```
  - 임시(ad-hoc) 서명(`cdhash` 기반 DR)이 **아니어야** 한다. `mac-build-dmg.sh` 출력에 `"지난 빌드와 DR이 같아요"`가 표시되어야 한다.

### 2.3 덮어쓰기·업데이트 뒤 권한 유지 실기
- [ ] 이전 판(`/Applications/Shookshok.app`)에서 **화면 기록** 및 **손쉬운 사용** 권한이 켜진 상태 확인.
- [ ] 새 판 `.app`으로 덮어 설치(또는 자동 업데이트 적용) 후 재실행 → **권한을 다시 켜지 않고도** F1 범위 캡처·요소 강조가 즉시 동작하는지 확인.
- [ ] 권한이 의도적으로 꺼진 상태(`tccutil reset` 또는 새 환경)에서 첫 실행 권한 카드(② 화면 기록 / ③ 손쉬운 사용)와 설정 › 일반 › 권한 점검이 정상 안내·복구되는지 확인.
- [ ] 미공증 앱 첫 실행 안내([docs/handoff/mac-self-sign.md](../handoff/mac-self-sign.md) §3: 시스템 설정 › 개인정보 보호 및 보안 › **그래도 열기**)가 최신 macOS UI와 일치하는지 확인.

---

## 3. macOS 자동 업데이트 & 릴리스 초안 게시 (`R2-UPD`)

> **구현 구조 참고**: 맥 자동 업데이터(`crates/ink-platform-macos/src/updater.rs`, `crates/tauri-plugin-screen-ink/src/update.rs`)는 외부 HTTP 크레이트 없이 macOS 기본 도구(`/usr/bin/curl`·`shasum`·`ditto`·`codesign`)만 사용한다. `latest-mac.json`의 다운로드 주소는 `https://github.com/djqanrudgns12/screen-ink/releases/download/`만 허용하고, SHA-256 해시와 **현재 설치된 앱의 지정 요구 사항(DR) 일치 여부**를 `codesign --verify -R=<DR>`로 검증한다.

- [ ] `scripts/release-mac.sh`로 **초안(Draft)** 릴리스 생성:
  ```bash
  APPLE_SIGNING_IDENTITY="Shookshok Self Sign" sh scripts/release-mac.sh \
    "바뀐 점 1" "바뀐 점 2" "바뀐 점 3"
  ```
- [ ] `target/release/upload/` 및 GitHub 초안에 파일 3종이 모두 올라갔는지 확인:
  - `Shookshok_<버전>_aarch64.app.zip`
  - `Shookshok_<버전>_aarch64.dmg`
  - `latest-mac.json` (`version`, `notes` 최대 3줄, `url`이 정확한 태그 자산 URL, `sha256` 64자리 소문자 hex 일치)
- [ ] 로컬 업데이트 스테이징·서명 검증 회귀 시험 통과:
  ```bash
  cargo test -p ink-platform-macos updater::tests::실제_설치판으로_받기_확인 -- --ignored --nocapture
  ```
- [ ] 변조 시나리오 방어 확인(단위 시험 `목록은_github_주소와_올바른_해시만` 및 무시 시험 내 해시 불일치 거절):
  - 다른 도메인 URL 거절 · 잘못된 SHA-256 거절 · 다른 서명 인증서 `.app` 거절.
- [ ] 알림 미룸 및 실패 격리 확인:
  - 그리기·캡처·녹화 중 또는 "오늘은 그만(`update.snooze`)" 상태에서 알림이 즉시 뜨지 않고 연기되는지 확인.
  - 오프라인·차단망에서 자동 확인 실패 시 앱 동작·성능 영향 0, 조용히 다음 주기로 넘어가는지 확인.
- [ ] **[사람 최종 승인]** GitHub 릴리스 초안에서 버전·노트·파일 3종·SHA-256을 확인한 뒤 **사람이 직접 `[Publish release]` 클릭** (에이전트가 자동 게시하지 않는다).

---

## 4. 네트워크 0 & 업데이트 끈 상태 통신 0 (`R-NET-1·2` · `R2-UPD-4`)

- [ ] **Rust 런타임 HTTP 크레이트 금지 (`R-NET-2`)**:
  ```bash
  cargo tree --workspace --edges normal | grep -E '\b(reqwest|ureq|hyper|surf|isahc|curl|tauri-plugin-http|tauri-plugin-updater)\b' || echo "OK: 0건"
  ```
  - 참고: `Cargo.lock`에 `tauri`의 비활성 선택 피처 메타데이터가 남을 수 있으므로 반드시 활성 의존 그래프(`cargo tree --edges normal`)와 릴리스 바이너리 심볼/Import로 최종 판정한다.
- [ ] **웹뷰 CSP 확인 (`apps/screen-ink/src-tauri/tauri.conf.json`)**:
  - `connect-src`에 외부 `http://`·`https://` 도메인이 없는지 확인 (로컬 `ipc:`·`ink:`만 허용; 개발용 `ws://localhost:1420`은 릴리스 빌드에서 외부 노출 없는지 확인).
- [ ] **업데이트 끈 상태(`update.auto = false`) 통신 0 실측 (`R2-UPD-4`)**:
  - 설정 › 일반 › 업데이트에서 **자동 업데이트 확인 끄기** 설정 후 앱 재실행.
  - macOS: `SCREEN_INK_UPDATE_DELAY_SECS=5`로 릴리스 `.app` 실행 후 30초 이상 관찰하며 `lsof -i -a -p <PID>` 및 `nettop -p <PID>`에서 외부 소켓 생성·송수신 바이트 **0** 확인.
  - Windows: MSIX/비패키지 모두 `update.auto = false` 또는 비패키지 경로에서 앱 프로세스 외부 연결 **0** 확인.
- [ ] **Windows PE Import 검사 (1차 7.5)**:
  - `target/release/screen-ink.exe` (또는 `Shookshok.exe`)에서 `winhttp.dll` · `wininet.dll` · `ws2_32.dll` import가 없는지 `dumpbin /imports` / `llvm-objdump -p` / `rea inspect-pe-imports`로 확인.
  - WebView2 런타임 백그라운드 통신 억제 옵션 적용 여부 기록.
- [ ] 학교망(SSL 검사)·오프라인 환경에서 앱 실행·캡처·그리기·단축키 PDF 저장 시 오류·지연 0건 확인.

---

## 5. 개인정보 보호 & 기록에 창 제목 0 (`R-LOG-1` · 2차 원칙 5)

- [ ] **로그 카나리아 시험(창 제목·파일명·사용자명 0건)**:
  - 창 제목에 고유 카나리아 문자열(예: `SCREEN_INK_CANARY_TITLE_99`)이 포함된 시험 창을 띄운 뒤 F1(범위)·F2(창)·F3(녹화)·Alt+1(그리기)·글꼴 고르기·트레이 클릭을 1회씩 수행.
  - 로그 폴더(`~/Library/Logs/com.ssamuniverse.shookshok/Shookshok.log` 또는 `%LOCALAPPDATA%\...\Shookshok.log`) 전체 검색:
    - 카나리아 창 제목: **0건**
    - 사용자 계정명/홈 경로 안 개인 파일명: **0건**
    - 이 컴퓨터 시스템 글꼴 이름(`system_fonts` 응답): **0건** (화면 목록에만 쓰고 로그·파일에 기록하지 않음)
- [ ] 로그 순환·기본 레벨 확인 (`apps/screen-ink/src-tauri/src/lib.rs`):
  - `SCREEN_INK_LOG` 미설정 시 기본 `Info` 레벨, 파일 상한 2 MB, `RotationStrategy::KeepOne` 동작 확인.
  - 트레이 클릭 계측(`tray click: ...`)은 숫자·참거짓만 남고 창 제목이 없는지 확인.
- [ ] 출력물 개인정보 0건 확인:
  - 단축키 PDF(`슉쇽-단축키-*.pdf` / `shookshok-shortcuts-*.pdf`) 텍스트 추출 시 창 제목·사용자 이름·경로 0건.
  - 캡처/그리기 저장 PNG에 불필요한 EXIF/tEXt 메타데이터 없음, 불투명 이미지 알파 채널 제거 확인(`R-OUT-3`).
  - 범위 기록(`capture.history`)에 이미지·창 제목 없이 사각형 좌표만 저장되는지 확인.

---

## 6. 글꼴 라이선스 고지 & 번들 글꼴 검증 (`R2-FONT-2·3`)

### 6.1 포함 글꼴 및 라이선스 대조표
현재 저장소(`packages/ink-ui/src/lib/fonts/`)에 포함된 글꼴과 라이선스 원문(`LICENSES/`):

| 글꼴 | 용도 | 포함 파일 | 용량 합계 | 라이선스 | 원문 파일 |
|---|---|---|---:|---|---|
| **Freesentation** | UI 본문 (400·500·600) + PDF 본문 (400·600) | `Freesentation-{4Regular,5Medium,6SemiBold}.woff2`, `pdf/Freesentation-{4Regular,6SemiBold}.ttf` | 약 1,096 KiB | SIL OFL 1.1 (© 2024 PT&) | `LICENSES/Freesentation-OFL.txt` |
| **Paperlogy** | 트레이·제목 (500·600) + PDF 머리 (600) | `Paperlogy-{5Medium,6SemiBold}.woff2`, `pdf/Paperlogy-6SemiBold.ttf` | 약 556 KiB | SIL OFL 1.1 (© 2024 PT&) | `LICENSES/Paperlogy-OFL.txt` |
| **IBM Plex Mono** | 숫자·키캡 (400·500) + PDF 키캡 (500) | `@fontsource/ibm-plex-mono` woff2 + `pdf/IBMPlexMono-Medium.ttf` | 약 55 KiB | SIL OFL 1.1 (© 2017 IBM Corp.) | `LICENSES/IBMPlexMono-OFL.txt` |
| **Pretendard** | PDF macOS 기호 (`⌥⌘⇧⌃` 등 54자) | `pdf/Pretendard-Symbols.ttf` | 약 21 KiB | SIL OFL 1.1 (© 2021 Kil Hyung-jin) | `LICENSES/Pretendard-OFL.txt` |

### 6.2 릴리스 전 필수 점검
- [ ] **앱 내 고지 일치 확인 (`설정 › 정보`, `about.opensource.fontsSub`)**:
  - **주의(현재 확인된 보완 필요 항목)**: `packages/ink-ui/src/lib/i18n/areas/about.{ko,en}.json`의 `about.opensource.fontsSub`에는 현재 `Freesentation · Paperlogy · IBM Plex Mono (SIL OFL 1.1)`만 적혀 있고, PDF에 임베딩되는 **`Pretendard`**가 빠져 있다. 릴리스 전 `Pretendard`를 고지에 추가했는지 확인한다.
- [ ] `packages/ink-ui/src/lib/fonts/LICENSES/` 4개 OFL 전문 파일이 모두 유효하고 배포/저장소에서 접근 가능한지 확인.
- [ ] 임베딩·수정 금지 글꼴(`Eulyoo1945`, `Bookk` 등)이 UI·PDF·에셋 어디에도 포함되지 않았는지 확인.
- [ ] 글꼴 부분 집합·커버리지 검사 통과 (`packages/ink-ui/src/lib/fonts/fontCoverage.test.js`):
  - 카탈로그(ko·en)의 모든 글자 빠짐 0건.
  - `Paperlogy`의 빈 한글 글리프 0건 (`manifest.json` 및 `pdf/manifest.json` 모두 `emptyHangulCount: 0`).
  - 앱 번들 내 글꼴 총 크기 예산 준수 (현재 UI 642 KiB + PDF 1,094 KiB = 약 1.70 MiB ≤ 2 MB).

---

## 7. macOS `Info.plist` 권한 문구 · 다국어 표시 이름 (`R2-ONB-8`)

- [ ] `apps/screen-ink/src-tauri/Info.plist` 및 빌드된 `Shookshok.app/Contents/Info.plist` 검사:
  - `CFBundleIdentifier` = `com.ssamuniverse.shookshok`
  - `CFBundleShortVersionString` / `CFBundleVersion` = 릴리스 판 번호
  - `CFBundleDisplayName` = `슉쇽` (번들 폴더명은 `Shookshok.app`)
  - `LSMinimumSystemVersion` = `14.0` (`apps/screen-ink/src-tauri/tauri.macos.conf.json`)
- [ ] **다국어 앱 표시 이름 (`InfoPlist.strings`) 확인**:
  - **주의(현재 확인된 보완 필요 항목)**: 현재 저장소에는 `Info.plist` 단일 파일(`CFBundleDisplayName = 슉쇽`)만 있고 `ko.lproj/InfoPlist.strings`·`en.lproj/InfoPlist.strings`(`ShookShok`)가 아직 번들에 포함되지 않았다. 영어 macOS에서 앱 이름이 `ShookShok`으로 보이도록 다국어 리소스 포함 여부를 확인한다.
- [ ] **권한 설명 문구 확인**:
  - **화면 기록(Screen Recording)·손쉬운 사용(Accessibility)**: macOS는 앱별 `Info.plist` 설명 문구 키를 별도로 받지 않는다. 대신 첫 실행 권한 카드(②·③)와 설정 › 일반 › 권한 점검에 한국어·영어 사유 안내가 빠짐없이 표시되는지 확인.
  - **마이크 녹음(`NSMicrophoneUsageDescription`)**: macOS 녹화에서 마이크 입력 기능을 켤 수 있게 된 판부터는 `Info.plist`(및 `ko.lproj`/`en.lproj` `InfoPlist.strings`)에 `NSMicrophoneUsageDescription`을 **반드시** 넣어야 한다(누락 시 마이크 접근 순간 macOS TCC가 프로세스를 즉시 종료함). 마이크 입력이 비활성인 판이라면 녹화 경로에서 오디오 캡처 장치를 열지 않음을 확인.
- [ ] 사용하지 않는 민감 권한 키(`NSCameraUsageDescription`, `NSLocationUsageDescription`, `NSContactsUsageDescription`, `NSAppleEventsUsageDescription` 불필요분)가 없는지 확인.

---

## 8. Windows MSIX · 스토어 · 직접 배포 · 비패키지 (1차 7.5·7.6 + H 시안)

### 8.1 MSIX 포장 및 매니페스트 (`D2` · `D3`)
- [ ] `apps/screen-ink/src-tauri/packaging/msix/` 설정 일치 확인:
  - `AppxManifest.template.xml`: `Identity Name`, `Publisher`, 4자리 `Version`(`X.Y.Z.0`), `ProcessorArchitecture="x64"`, `TargetDeviceFamily MinVersion="10.0.19041.0"`, `runFullTrust`, `windows.startupTask`(`ShookshokStartup`), WGC 기능 선언.
  - `msix.config.json`: 표시 이름(`슉쇽 (ShookShok)`), 게시자 표시 이름(`찰떡쌤`), 실행 파일명 일치.
  - `apps/screen-ink/src-tauri/icons/` 타일·스토어 PNG 자산 전 크기 누락 없음.
- [ ] 패키지 모드(`MsixStore` / `MsixDirect`) 런타임 분기 확인:
  - `Shookshok.portable` / `screen-ink.portable` 표시 파일 무시 (설치 폴더 읽기 전용).
  - 로그인 자동 실행이 `StartupTask("ShookshokStartup")`로 동작하고, 사용자가 Windows 설정에서 끈 경우(`DisabledByUser`) 설정 › 일반에 한 줄 안내 표시.
  - 설정 › 일반 › 설정 위치에 `"MSIX 패키지"` 표기.

### 8.2 Microsoft Store 채널 (`MsixStore` · `D5` · `D6`)
- [ ] 스토어용 무서명 `.msix` 빌드·검증 통과: `npm run msix:store` (`-Verify`).
- [ ] `Publisher`가 Partner Center 발급 `CN=...`과 정확히 일치.
- [ ] **WACK (Windows App Certification Kit)** 로컬 실행 통과.
- [ ] 스토어 등록 자료([docs/store/microsoft-store-listing.md](../store/microsoft-store-listing.md)) 최신화(2차 용어집 말투, 2차 화면 스크린숏, 심사 메모: `runFullTrust`·전역 단축키·접근성 영역 감지·오프라인 동작 사유 기재).
- [ ] 스토어 공개 후 `winget install <StoreID> --source msstore` 설치·실행 확인.

### 8.3 직접 배포 MSIX & `.appinstaller` (`MsixDirect` · `D8` · `D9` · `D11`)
- [ ] 루트 `README.md`의 `## Code Signing Policy` 절(SignPath Foundation 필수 문구) 유지 확인.
- [ ] `.github/workflows/release-msix.yml`에서 SignPath 서명된 `.msix` + `shookshok.appinstaller` 생성·검증 통과 (`Publisher`가 SignPath 인증서 주체와 글자 단위 일치).
- [ ] `shookshok.appinstaller`의 패키지 `Uri`가 `latest` 리디렉션이 아닌 **버전 고정 태그 URL**(`.../releases/download/v<ver>/...`)을 가리키는지 확인.
- [ ] 다른 Windows PC에서 브라우저로 `.msix` / `.appinstaller`를 내려받아 실행 → 인증서 수동 설치 없이 `[설치]` 진행 및 SmartScreen 경고 여부 기록.
- [ ] 이전 판 → 새 판 `.appinstaller` 업그레이드 실기: `settings.json` 유지, `RegisterApplicationRestart` 자동 재시작 확인.

### 8.4 비패키지(NSIS · 포터블 ZIP) & 보안·호환 (1차 7.5·7.6)
- [ ] **UPX 패킹 절대 금지**: PE 섹션에 `UPX0`·`UPX1` 없음 확인.
- [ ] **표준 사용자 계정**(관리자 권한 없음)에서 NSIS `currentUser` 설치·실행·전역 단축키·제거가 깨끗하게 동작.
- [ ] **포터블 ZIP**(`Shookshok.exe` + 0바이트 `screen-ink.portable` 또는 `Shookshok.portable` + `LICENSE`)을 다른 폴더로 옮겨 실행 시 같은 폴더 `data/`만 사용하고 `%APPDATA%`에 쓰지 않음 확인.
- [ ] WebView2 런타임 부재 환경 안내(`MessageBoxW`) 확인.
- [ ] Windows Defender(최신 정의) 검사 및 오탐 0건 확인.

---

## 9. 두 OS 사람 실기 최종 확인

### 9.1 macOS 실기
- [ ] **트레이 팝오버 전체 화면 앱 위 재발 확인 (`R2-BUG-TRAY`)**: 설정 창을 열어 둔 채 전체 화면 앱(예: 브라우저/Claude/Keynote) 위에서 메뉴 막대 아이콘 클릭 10회 중 10회 열림 + 로그 `tray click:` 줄 정상 기록.
- [ ] **캡처 글 상자 한글 조합 (`R2-TXT-0`)**: F1 → 범위 확정 → 글자 도구(`T`) → `"안녕하세요"` 한글 조합 입력 정상(숫자만 찍히거나 조합 깨짐 없음) + 복사/저장 결과에 고른 글꼴로 정상 굽힘.
- [ ] **단축키 바꾸기 F 키 (`R2-KEY-REC-3`)**: fn+F1 포함 F 키 녹화 중 전역 키가 오동작하지 않고, 녹화 취소·창 닫힘 후 기존 전역 키가 정상 복구됨.
- [ ] **단축키 PDF (`R2-PDF-5`)**: 설정 › 단축키 `[PDF로 받기]` → 저장 섬 `[열기]` → 미리보기에서 글자 검색·맥 기호(`⌥⌘⇧⌃`)·줄바꿈·쪽 넘김 이상 없음.
- [ ] **언어 즉시 전환 (`R2-I18N-2`)**: 설정 › 일반에서 한국어 ↔ 영어 전환 시 재시작 없이 설정·트레이·창 제목·다음 섬이 모두 즉시 변경됨.

### 9.2 Windows 실기
- [ ] **100% + 150% 혼합 배율 (`M7` · `S10`)**: 범위 캡처 정확도 0 px, 선택 툴바·색·굵기 패널·글꼴 고르기·핀 창 위치 정상.
- [ ] **실시간 확대(`Alt+2`) · 창 항상 위(`Alt+Shift+D`) · 녹화(`F3`)**: 1080p ≥ 30 fps, 녹화/캡처 결과에 자기 섬·테두리 0 px.
- [ ] **전체 화면 앱 위 트레이 팝오버(`R2-BUG-TRAY-5`)** 및 **업데이트 알림 미룸(`D10`)** 정상.
- [ ] **단축키 PDF (`R2-PDF-5`)**: 설정 › 단축키 `[PDF로 받기]` → Edge에서 글자 검색·Windows 키(`Ctrl·Alt·Shift·Win`) 표시 이상 없음.

---

## 10. 릴리스 확인 서명 표

| 항목 | 값 / 결과 | 확인자 | 일시 |
|---|---|---|---|
| 릴리스 판 번호 / 태그 | `v0.2.0` (예정) | | |
| 기준 커밋 SHA | | | |
| macOS 서명 신원 & DR 일치 | `Shookshok Self Sign` / 일치(예·아니오) | | |
| macOS `.app.zip` SHA-256 | | | |
| Windows MSIX (store / direct) SHA-256 | | | |
| 업데이트 끈 상태 통신 0 실측 | macOS / Windows | | |
| 로그 카나리아(창 제목 0건) | macOS / Windows | | |
| 자동 스냅숏·성능 예산 통과 | [snapshots.md](../qa/2026-11-27/snapshots.md) · [perf.md](../qa/2026-11-27/perf.md) | | |
| 최종 게시 승인 (GitHub / Store) | 초안 확인 후 사람 게시 | | |
