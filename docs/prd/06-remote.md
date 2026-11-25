# 06. 원격 기능 — PostHog 사용 분석 · 깃허브 릴리스 자동 업데이트 (2차)

2026-11-25 · PRD v3.0 부속 · **1차 빌드는 네트워크를 쓰지 않는다**(R-NET-2 그대로). 다만 구조는 지금 잡아 둔다: `Telemetry`·`Updater` 포트를 1차부터 두고 `Noop` 구현을 끼운 뒤, 1차 구현을 마친 다음 `ink-remote` 어댑터만 바꿔 끼운다([03-architecture](03-architecture.md) 3절). 설정 화면의 "연결" 묶음 자리도 이미 있다(S05).

두 기능 모두 **Tidy-Task-Local에 이미 검증된 구현이 있어 그대로 옮긴다**(Tidy 조사, 2026-11-25).

## 1. PostHog 사용 분석

### 1.1 Tidy 엔진을 그대로 쓴다

| 항목 | Tidy 구현(근거) | screen-ink |
|---|---|---|
| 보내는 쪽 | Rust `reqwest`로 `{host}/batch/` POST, 모든 창이 엔진 하나 공유 (`analytics.rs:72-83, 440-556`) | 같음. JS는 `invoke('analytics_track')`만 |
| 익명 ID | OS 난수 UUID v4, 앱 데이터의 `*-analytics.json` (`:22-36, 445-449`) | 같음(`screen-ink-analytics.json`) |
| 이벤트 모양 | **닫힌 허용 목록**, 속성은 `count`(범위 제한)와 정해진 `choice`만 (`:141-252`) | 같음(아래 1.2) |
| 공통 속성 | `app_version` `os` `arch` `schema_version` `environment` `$session_id` `window_kind` | 같음 + `monitors`(개수) `dpi_mix`(같음/섞임) |
| 개인정보 | `$process_person_profile=false`, `$geoip_disable`, `$ip=0.0.0.0`, 본문·이름·경로·오류 원문 없음 (`:126, 140, 300-302`) | 같음 + **창 제목·파일 이름·화면 내용·실행 중 앱 목록 없음** |
| 큐 | 디스크 최대 1000건·7일, 15초마다 50건, 실패 시 2배씩 최대 900초, 400/401/403/404 = 설정 오류 (`:12-13, 283-289, 497-548`) | 같음 |
| 키 | 빌드 때 `option_env!`: `POSTHOG_PROJECT_TOKEN` `POSTHOG_HOST` `POSTHOG_DEV_ENABLED`, `build.rs`가 release에 토큰 강제 (`build.rs:3-27`) | 같음. 토큰은 저장소에 두지 않는다(`.env`는 `.gitignore`) |
| 학교망 | `reqwest` **native-tls**(Windows 인증서 저장소) | 같음 — 학교 SSL 검사 장비의 루트 인증서를 OS가 믿으면 통과. Snipaste(OpenSSL 동봉)는 이 PC에서 SSL 악수 실패 95회 [원인 추정] |
| Tidy 안에 넣을 때 | — | **Tidy 엔진 하나만 돈다**: `window_kind`에 `ink-*` → `screen_ink`, 허용 표에 아래 이벤트 추가 |

### 1.2 screen-ink 허용 이벤트 (초안)

| 이벤트 | 속성(`choice`/`count`만) | 알고 싶은 것 |
|---|---|---|
| `app_started` · `session_started` · `app_active` | (Tidy 공통) | 사용자 수·지속 사용 |
| `capture_completed` | `mode`: element·window·drag·full·last·history · `result`: copy·save·both · `level_moves`: 0·1·2-3·4+ · `magnet_used`: yes/no · `suggestion_shown`: none·color·table·text·qr | 스마트 인식이 실제로 쓰이는지, 휠 계층 이동이 필요한지 |
| `capture_cancelled` | `after_ms`: <1s·1-5s·5s+ | 잘못 누름·범위 불만 |
| `window_capture_completed` | `trigger`: click·double_tap · `fallback`: wgc·printwindow·bitblt · `blocked`: yes/no | WGC 실패율·보호 화면 빈도 |
| `record_finished` | `duration`: <1m·1-5m·5-15m·15m+ · `fps`: 30·60 · `mic`: yes/no · `stop`: user·lock·sleep·display | 녹화 길이 분포·자동 중지 빈도 |
| `draw_session` | `strokes`: 0·1-5·6-20·21+ · `zoom_used` · `pan_used` · `shapes_corrected`: count · `kept`: yes/no · `restored`: yes/no | 판서 습관, 보정·보관 쓰임 |
| `live_zoom_used` | `max_zoom`: 1.5·2-3·4+ · `to_draw`: yes/no | Alt+2 → Alt+1 전환 쓰임 |
| `board_used` | `background`: white·black | |
| `shortcut_conflict` | `mode` · `culprit`: snipaste·icanzoomit·zoomit·snipping_tool·unknown · `resolved_by`: suggestion·manual·kept | 기본 키 정책(F1 등) 결정 근거 |
| `settings_applied` | `keys`: 바뀐 설정 이름 목록(정해진 이름만) | 기본값 조정 |
| `perf_daily` | `overlay_ready_p50`·`p95`: 구간(≤50·≤80·≤150·≤300·300+) · `launch_p50` 구간 · `monitors` · `dpi_mix` | **학교 PC 전체의 M1 분포**(벤치마크를 현장으로 확장) |
| `app_error` | `area`: capture·record·hotkey·settings·overlay · `code`: 정해진 오류 코드 | 오류 원문 없이 빈도만 |

보내지 않는 것: 이미지·창 제목·창 안 글자·파일 이름·경로·저장 폴더·실행 중 앱 목록(충돌 원인은 위 enum으로만)·키 입력 내용·좌표.

### 1.3 동의

- Tidy는 동의 화면이 없고 토큰이 있으면 켜지며, 도움말 "사용 통계 안내"로 알린다(`analytics.rs:277-282`, `help.html:665`).
- screen-ink 제안: **첫 실행 카드(S06)에 한 줄 안내 + 일반 > 연결 "익명 사용 통계 보내기" 스위치 + "보낸 정보 보기"**(최근 50건을 사람이 읽는 말로). 끄면 큐를 비우고 익명 ID를 새로 만든다.
- 기본값(켜짐/꺼짐)은 **사용자 결정 사항**(PRD 10절). 학교 개인정보 검토를 고려하면 "켜짐 + 분명한 안내 + 쉬운 끄기"가 Tidy와 맞고, "꺼짐(동의 후 켜짐)"이 가장 보수적이다.

## 2. 깃허브 릴리스 자동 업데이트

### 2.1 Tidy 구조를 그대로 쓴다

| 항목 | Tidy 구현(근거) | screen-ink |
|---|---|---|
| 엔진 | `tauri-plugin-updater` 2.12, 기능 `native-tls`·`system-proxy` (`Cargo.toml:37-39`), 순서는 `app_update.rs`가 직접 지휘 | 같음(학교 프록시·SSL 검사 대응) |
| 새 버전 찾기 | GitHub API `releases/latest`(인증 없음), 시작 2초 뒤 + 30분마다 (`updateChecker.js:14-19`, `appState.svelte.js:937-971`) | 시작 2초 뒤 + 6시간마다. **모드(캡처·판서·녹화) 중에는 확인도 미룸** |
| 매니페스트 | `…/releases/latest/download/latest.json`, `{version, pub_date, platforms{windows-x86_64(-nsis): {signature, url}}}` (`tauri.conf.json:44-46`, `scripts/lib/updaterRelease.mjs`) | 같음 |
| 서명 | minisign 업데이터 키 — 공개 키는 `tauri.conf.json`, 개인 키는 저장소 밖 `~/.tauri/*.key` | 같음(키는 새로 만듦: `~/.tauri/screen-ink-updater.key`) |
| 설치 | 사용자가 누를 때: 받기(취소 가능) → 창마다 저장 확인 응답(15초) → 사본 → 쓰기 잠금 → `install()` passive (`app_update.rs:381-434`) | 같음 + **녹화 중이면 설치 단추 비활성("녹화를 마치면 설치할 수 있어요")**, "끌 때 설치" 선택지 추가 |
| 실패 | 확인 20초·연결 20초·조각 30초 제한, 코드별 한국어 안내, 안 되면 브라우저로 직접 받기 | 같음 |
| 나중에·건너뛰기 | 24시간 미루기 · 그 버전 건너뛰기 (`updateChecker.js:186-198`) | 같음 |
| 릴리스 | `npm run release`(버전 동기화 · latest.json 생성 · 서명 검증) | Tidy `scripts/lib/updaterRelease.mjs`·`minisign.mjs`를 옮겨 씀 |

### 2.2 꼭 먼저 정할 것: 저장소 공개 여부

Tidy 업데이터는 **공개 저장소**를 전제한다(토큰 처리 없음, `업데이트-배포-가이드.md:212`). screen-ink 저장소는 지금 **비공개**라 그대로는 앱이 릴리스를 읽을 수 없다. 선택지:

| 안 | 내용 | 장단 |
|---|---|---|
| A | screen-ink 저장소를 공개 | 가장 단순, 오픈소스 무료 서명 프로그램 신청 가능 / 코드 공개 |
| **B (추천)** | 코드는 비공개 유지 + **공개 릴리스 전용 저장소**(`screen-ink-releases`)에 설치 파일·`latest.json`만 올림 | 코드 비공개 유지, Tidy 업데이터 코드 거의 그대로(저장소 이름 상수만) |
| C | 앱에 읽기 토큰 넣기 | **금지** — 토큰이 실행 파일에서 새어 나감 |

### 2.3 배포 형태별

| 형태 | 업데이트 |
|---|---|
| 설치판(NSIS) | 위 흐름 그대로 |
| 포터블 ZIP | 자동 설치 안 함 → "새 버전이 있어요 [받으러 가기]"(설정·데이터 폴더 유지 안내) |
| MS Store | 앱 안 업데이트 끔(Store가 갱신) — Tidy는 같은 EXE를 Store에 내는 안을 권장(`MS-스토어-게시-가이드.md:27`) |
| Tidy 안 | Tidy 업데이터가 담당. screen-ink는 `update-prepare`를 받으면 모드를 끝내고 응답(녹화 중이면 마무리 후 응답) |

## 3. 2차 작업 순서 (1차 구현을 마친 뒤)

1. `ink-remote` 크레이트: Tidy `analytics.rs`를 옮겨 허용 표·`window_kind`만 바꿈 → `Telemetry` 포트 구현. `cargo test`(큐·백오프·허용 목록 거부).
2. `build.rs`: 토큰 주입·release 강제(Tidy 복사), `.env.example`에 키 이름만.
3. 설정 > 연결: 스위치 활성화, "보낸 정보 보기" 대화.
4. 첫 실행 카드에 안내 한 줄(1.3 결정에 따라 문구).
5. 저장소 결정(2.2) → 업데이터 키 만들기 → `tauri.conf.json` `plugins.updater`(endpoint·pubkey) → `Updater` 포트 구현(Tidy `app_update.rs` 이식, 녹화 가드 추가).
6. 릴리스 스크립트 이식 → 시험 릴리스 0.1.1 → 0.1.2로 실제 업데이트 확인(학교망·표준 사용자).
7. PostHog 대시보드: 위 이벤트로 기본 인사이트(일일 사용자, 모드별 사용, M1 분포, 충돌 원인) — PostHog 쪽 작업은 연결 권한이 필요(이 세션의 PostHog 연결은 인증 전).

## 4. 1차에서 미리 지키는 것

- 포트는 `Noop`으로 연결, 1차 빌드에 `reqwest`·updater 플러그인이 **들어가지 않음**을 `cargo tree`로 확인(R-NET-2).
- 이벤트를 보낼 자리(유스케이스 끝)에 `ports.telemetry.track(Event::CaptureCompleted{..})` 호출을 미리 넣어 둔다 — 1차에서는 아무 일도 안 하지만, 2차에서 빠뜨림이 없다.
- 계측 줄(R-BENCH-1)과 `perf_daily`는 같은 값을 쓴다(로컬 로그 = 현장 통계).
