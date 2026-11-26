# 슉쇽(ShookShok) 영어 지원(i18n) 화면 문자열 전수 조사 및 구현 제안

- **기준 브랜치 / 작업 폴더**: `feat/foundation` (`/Users/bodyking/Desktop/Architecture/screen-ink`)
- **작성일**: 2026-11-27
- **상세 목록 CSV**: [i18n-strings.csv](i18n-strings.csv) (`파일:줄`, `층`, `원문`, `동적 값 여부`, `제안 키` — 총 **1,270행**, 고유 키 **962개**)
- **관련 PRD**: [PRD-2차-고도화.md](../PRD-2차-고도화.md) (§2 `R2-I18N-1..8`)

---

## 1. 조사 범위, 선반영 현황 및 제외 기준

### 1.1 조사 범위
- **화면(`ui`)**: `packages/ink-ui/src/lib/**` (`.svelte`, `.js` — `components/`, `screens/`, `models/`, `ink/`, `icons/icons.js`), `apps/screen-ink/src/**`, `packages/ink-client/src/{index,fake}.js`.
- **Rust 섬·인식(`rust-island`)**: `crates/ink-app/src/{capture,draw,live_zoom,quick,record,service,state,window_capture}.rs`, `crates/ink-domain/src/{recog,selection,zoom}.rs`, `crates/tauri-plugin-screen-ink/src/lib.rs`(섬 이벤트).
- **Rust 오류·안내·충돌(`rust-error`)**: `crates/ink-domain/src/{naming,package,settings,shortcuts}.rs`, `crates/ink-app/src/{ports,service}.rs`, `crates/ink-store/src/lib.rs`, `crates/ink-platform-windows/src/powertoys.rs`, `crates/tauri-plugin-screen-ink/src/lib.rs`(파일 읽기 오류), `apps/screen-ink/src-tauri/src/lib.rs`.
- **트레이·네이티브 메뉴(`tray`)**: `crates/tauri-plugin-screen-ink/src/{lib,pins}.rs`. (참고: 트레이 팝오버 본체는 Svelte WebView `TrayPopoverScreen.svelte`로 구현돼 `ui` 층에 집계)
- **OS 번들·창 제목·대화상자(`os`)**: `apps/screen-ink/src-tauri/{Info.plist,tauri.conf.json,tauri.macos.conf.json}`, `apps/screen-ink/index.html`, `crates/tauri-plugin-screen-ink/src/{lib,overlay,pins,rec_frame,zoom_hud}.rs`.
- **데이터(`data`)**: `docs/shortcuts/catalog-v1.json` (`slots`, `sources`, `rules.imeKeys`, `refs[].what`).
- **스토어·MSIX(`store`)**: `apps/screen-ink/src-tauri/packaging/msix/{msix.config.json,AppxManifest.template.xml}`, `docs/store/microsoft-store-listing.md`.

### 1.2 작업 트리 내 선반영 i18n 발판
조사 시점 기준으로 다음 기초 코드가 이미 착수되어 있다. 제안 키는 이 기존 키 8개와 100% 호환되게 맞췄다.
- 공용 카탈로그: `packages/ink-ui/src/lib/i18n/ko.json`, `packages/ink-ui/src/lib/i18n/en.json` — **8개 키** 선반영 (`settings.tab.{shortcuts,capture,draw,general,help}`, `island.hotkeys.{on,onSome,off}`).
- Rust: `crates/ink-domain/src/i18n.rs` (`include_str!`로 위 두 JSON 공유, `Lang::{Ko,En}`, `tr`, `trf`, `placeholders`, 키/자리표시자 일치 단위 시험), `crates/ink-app/src/service.rs:632-671` (`lang()`, `t()`, `tf()`, 단축키 켜기/끄기 섬 3건 연결).
- UI: `packages/ink-ui/src/lib/i18n/{core.js,index.svelte.js}` (`t`, `syncLang`, `setLang`), `SettingsWindow.svelte:14,38-43,73,79,158` (설정 탭 4건 연결).

### 1.3 제외 대상 (사용자 비노출)
- `log::{trace,debug,info,warn,error}!`, `debug_assert!`, 개발용 `.expect("직렬화")` 등 진단·로그 문자열.
- 단위/통합 테스트 전용(`#[cfg(test)]`, `*.test.js`, `crates/ink-vision/src/tests.rs`, `crates/ink-platform-fake/**`).
- 개발자용 미리보기 하네스(`packages/ink-ui/src/preview/{Preview,DrawPreview}.svelte`).
- `crates/ink-platform-windows/src/package.rs:183-637`의 WinRT/DLL 내부 오류 30건 — 호출부(`crates/tauri-plugin-screen-ink/src/lib.rs:667`)에서 `log::warn!`으로만 기록되고 UI로 전달되지 않아 제외했다.
- `crates/ink-store/src/lib.rs:72` `"schemaVersion 없음"` — `load()` 내부에서 `log::warn!` 뒤 사용자용 `Loaded.notice` 3건(`L140,144`)으로 치환되므로 `Loaded.notice`만 포함했다.

---

## 2. 층별 문자열 개수

| 층 (`layer`) | CSV 출현 건수 | 층 내 고유 키 | 동적 문구 행 | 주요 출처 및 비고 |
|---|---:|---:|---:|---|
| `ui` | **600** | **423** | 53 | 실제 앱 UI 495건(고유 347키) + `fake.js` 78건(미리보기 전용 7키) + `icons.js` 메타 27건(27키, 현재 Preview 부품 판에서만 표시) |
| `rust-island` | **197** | **161** | 35 | `ink-app` 섬·힌트 153건 + `ink-domain` 33건(`recog.rs` 30 + `selection` 2 + `zoom` 1) + `tauri-plugin-screen-ink` 섬 11건 |
| `rust-error` | **92** | **82** | 24 | `naming` 6 + `package` 9 + `settings` 21 + `shortcuts` 22 + `ports` 3 + `service` 11 + `ink-store` 3 + `powertoys` 14 + 플러그인/진입점 3 |
| `tray` | **8** | **8** | 2 | 네이티브 트레이 툴팁 4건(`lib.rs:46,367-372`) + 핀 우클릭 네이티브 메뉴 4건(`pins.rs:189`) |
| `os` | **14** | **11** | 5 | `Info.plist` 1 + `tauri.conf.json` 1 + `index.html`(`lang`/`title`) 2 + 네이티브 창 제목 8건 + 파일 열기/저장 필터 2건 |
| `data` | **329** | **329** | 0 | `catalog-v1.json`: `slots` 57 + `sources` 12 + `imeKeys` 2 + `refs[].what` 258(고유 문구 233개) |
| `store` | **30** | **30** | 1 | MSIX `msix.config.json`·`AppxManifest.template.xml` 4건 + Microsoft Store 등록 문구 26건(§3 영문 초안 기작성) |
| **합계** | **1,270** | **962** *(층 간 공유 82키 제외)* | **120** | **실제 앱 런타임 번역 대상 = 887키** (핵심 629키 + `catalog` 참조 258키; 선반영 8키 제외 시 **잔여 879키**) |

> **규모 해석**: `store`(30키)·`os` 번들/창메타(11키)·`icons.js` 부품판 메타(27키)·`fake.js` 데모 전용(7키)을 제외한 **앱 런타임 카탈로그(`ko.json`/`en.json`) 번역 규모는 887키**다. 그중 **1차 출시 필수 핵심 키는 629키**이며, 단축키 충돌 상세 설명(`catalog-v1.json`의 `refs[].what` 258건, 고유 문구 233개)은 별도 오버레이 파일로 분리 관리하면 작업량을 단계별로 나눌 수 있다.

---

## 3. 동적 문구 · 복수형 · 숫자/날짜 형식

### 3.1 동적 문구 유형 (총 120행)
1. **단순 변수 치환** (예: `{hex} 복사했어요`, `{exe} 빼기`, `투명도 {opacity}%`, `모니터 {n}`, `더 새 버전(v{version})이 만든 설정이라 읽기만 해요`, `모르는 설정이에요: {path}`).
2. **복수형(Singular / Plural) 분기가 필요한 문구 (영어 필수)** — 한국어는 `{n}개`로 단일 표현하지만 영어는 `1` vs `other`가 다르다. 현재 `i18n.rs`의 `trf()`와 `core.js`의 `translate()`에는 복수형 도우미가 없다.

| 제안 키 (또는 위치) | 한국어 원문 | 영어 분기 예시 (`one` / `other`) |
|---|---|---|
| `island.hotkeys.onSome` (`service.rs:660`, 선반영) | `단축키를 켰어요 · {n}개는 다른 앱이 쓰고 있어요` | `… · 1 is used by another app` / `{n} are used by other apps` *(현재 `en.json`은 복수만 있음)* |
| `island.settings.shortcutsRestoredPartial` (`service.rs:624`) | `기본값으로 되돌렸어요 · {n}개는 겹쳐서 그대로예요` | `1 kept due to conflict` / `{n} kept due to conflicts` |
| `island.settings.importedWithConflicts` (`service.rs:1124`) | `설정을 가져왔어요 · 단축키 {n}개는 겹쳐요` | `1 shortcut conflicts` / `{n} shortcuts conflict` |
| `island.history.cleared` (`tauri-plugin-screen-ink/src/lib.rs:1041`) | `캡처 기록 {n}개를 지웠어요` | `Cleared 1 capture` / `Cleared {n} captures` |
| `island.record.lowDisk` (`record.rs:268`) | `남은 공간이 {m}분 정도예요` | `About 1 min of space left` / `About {m} min of space left` |
| `island.record.unfinished.{one,other}` (`record.rs:464`) | `끝나지 않은 녹화가 있어요` / `{n}개 있어요` | 이미 `n == 1` 분기 존재 → `.one`/`.other` 그대로 대응 |
| `island.record.recovered.{one,other}` (`record.rs:501`) | `녹화를 저장했어요` / `녹화 {n}개를 저장했어요` | 이미 `done == 1` 분기 존재 → `.one`/`.other` 그대로 대응 |
| `island.record.movePartialFailed` (`record.rs:504`) | `{n}개는 옮기지 못했어요` | `Couldn't move 1 file` / `Couldn't move {n} files` |
| `tray.status.conflicts` (`TrayPopoverScreen.svelte:81`) · `tray.tooltip.conflicts` (`lib.rs:369`) | `키 {n}개 겹침` | `1 key conflict` / `{n} key conflicts` |
| `tray.tooltip.pinned` (`lib.rs:372`) | ` · 고정한 창 {n}` | ` · 1 pinned window` / ` · {n} pinned windows` |
| `settings.capture.historyMax.sub` (`CapturePage.svelte:288`) | `{n}개 보관` | `Keep 1 item` / `Keep {n} items` |
| `settings.meta.countItems` (`settingsMeta.js:62,64`, `GeneralPage.svelte:164`) | `{n}개` | `1 item` / `{n} items` (또는 맥락별 `1 window`/`{n} windows`, `1 app`/`{n} apps` 분리 권장) |
| `settings.general.importSheet.titleChanges` (`GeneralPage.svelte:232`) | `{n}개 항목이 바뀌어요` | `1 setting will change` / `{n} settings will change` |
| `settings.shortcuts.conflict.*` / `firstRun.conflict.*` (`shortcutDisplay.js:53-88`, `settingsMeta.js:149-170`) | `단축키 {n}개가 겹치고 있어요`, `{keys} 외 {extra}개` | `1 shortcut conflicts` / `{n} shortcuts conflict`, `{keys} and {extra} more` |

### 3.2 숫자 · 단위 · 날짜 · 파일 이름 형식
- **언어 중립(변경 불필요)**: 배율 `×2.0` / `×1.75`, 해상도 `1920×1080`, 두께 `4px`, 프레임 `30 fps`/`60 fps`, 경과 시간 `00:42`, 각도 `±4°`, 투명도 `100%`, 파일 크기 `MB`/`GB`, 내보내기 파일 날짜 `Shookshok-settings-YYYY-MM-DD.json`(`tauri-plugin-screen-ink/src/lib.rs:965`), 기본 템플릿 `Shookshok_{yyyy}{MM}{dd}_{HH}{mm}{ss}`(`settings.rs:54`).
- **현지화 필요 단위/제약 문구**: `0.5초`(`cheatSheet.js:212`, `settingsMeta.js:106` → `0.5s`), `7일 보관`(`GeneralPage.svelte:208` → `Kept for 7 days`), `200자까지`(`naming.rs:24`), `{max}자 안의 글`(`settings.rs:163`), `{max}개까지`(`settings.rs:191`).
- **파일 이름 규칙 토큰 (`crates/ink-domain/src/naming.rs:7,72,108`)**: 기본 토큰 표에 `"앱"`이 들어 있으며 파서는 이미 `{app}`(대소문자 무시)을 별칭으로 허용한다(`L72`). 영어 UI/도움말에서는 `{app}`을 노출하고, 한국어 설정과의 호환을 위해 `{앱}`과 `{app}` 양쪽 파싱을 계속 유지해야 한다. (`fake.js:12`의 `수업_{yyyy}{MM}{dd}_{n}`도 영문 미리보기에서는 `Class_{yyyy}{MM}{dd}_{n}` 등으로 분기)

---

## 4. 언어마다 규칙이 다른 곳 (단순 치환 불가 · 구조 변경 필요)

### 4.1 단축키 이름 읽기 (`spoken()`, `keycaps()`, `canonical_key`)
- **`crates/ink-domain/src/shortcuts.rs:205` `spoken(combo, platform)`**: Windows는 `Shift F2`, macOS는 `⇧F2`를 만든다. 수식키·영문/기능 키는 언어 중립이지만 다음 세 군데가 한국어에 묶여 있다.
  1. **한국어 전용 물리 키 이름**: `canonical_key`(`shortcuts.rs:144-145`)와 `vk_to_key`(`L283-284`), `catalog-v1.json:20` `imeKeys`가 `"한/영"`, `"한자"`를 그대로 돌려준다. 내부 정규 키/가속기 매핑은 유지하되, 사용자 표시용 `spoken_for(combo, platform, lang)` 또는 UI `keycaps()`에서 `en`일 때 `"Hangul"` / `"Hanja"`로 바꿔 보여 줘야 한다.
  2. **의사 키캡(마우스·방향키)**: 섬 힌트와 단축키 한 장(`service.rs:138-140`, `live_zoom.rs:348`, `cheatSheet.js:126-147,200,213`)이 키캡 슬롯에 `"휠"`, `"클릭"`, `"드래그"`, `"방향키"`, `"더블클릭"`, `"오른쪽 클릭"`, `"{modCtrl} 휠"`을 직접 넣는다. 이것들은 실제 키코드가 아니므로 번역 키(`island.hint.key.wheel` 등)로 빼야 한다.
  3. **빈 칸/녹화 상태어**: `"비움"`(`FirstRunSheet.svelte:122`, `GeneralPage.svelte:250,256`, `ShortcutsPage.svelte:205`, `settingsMeta.js:67,85,86`), `"입력 중"`(`ShortcutsPage.svelte:205`) → 영어 `"None"`/`"Unassigned"`, `"Press keys…"`.

### 4.2 한국어 조사(`josa` · `subjectParticle`)와 쪼개진 문장 조립
한국어 받침에 따라 조사를 붙이는 로직과 문장을 앞뒤 조각으로 쪼개 이어 붙이는 코드가 7곳 있다. 영어는 조사가 없고 주어·동사·목적어 어순이 반대이므로 **완전한 문장 템플릿으로 재구성**해야 한다.

1. **`crates/ink-domain/src/text.rs` `josa(word, "과", "와")`** → `shortcuts.rs:636` `format!("{name}{} 같은 키예요", josa(&name, "과", "와"))` → 영어: `"Same shortcut as {name}"`.
2. **`packages/ink-ui/src/lib/models/shortcutDisplay.js:14` `subjectParticle(word)`** (`이`/`가`):
   - `formatConflictBanner`(`L51-89`): `Banner.svelte`에 `{lead: app, message: "가 F1 키를 먼저 쓰고 있어요"}`로 넘겨 `<strong>{lead}</strong>{message}`로 이어 붙인다. 영어에서는 `{app}`이 항상 문장 맨 앞에 오지 않을 수도 있고, `<strong>` 뒤에 공백이 필요하다.
   - `formatFirstRunConflict`(`settingsMeta.js:130-171`): `{prefix, app, suffix}` 3조각으로 쪼개 `FirstRunSheet.svelte:148`에서 `{prefix}<strong>{app}</strong>{suffix}`로 붙인다(예: `F1 키는 ` + `Snipaste` + `가 쓰고 있어요`). 영어는 `<strong>{app}</strong> is using {key}`처럼 `app`이 앞으로 온다.
   - **해법**: `Banner` / `FirstRunSheet`가 `lead` + `message`를 기계적으로 붙이지 말고, 번역 문자열 안의 `{app}` 자리를 기준으로 분할해 `<strong>{app}</strong>`을 감싸는 공통 경량 포매터(`splitHighlight(t(key, params), '{app}')`)를 쓰면 언어별 어순을 100% 자유롭게 둘 수 있다.
3. **인라인 조사 하드코딩 5곳**:
   - `service.rs:564`: `format!("{}로 바꿨어요", spoken(...))` (`F1`은 받침 없음/있음에 따라 `로`/`으로` 자체가 한국어에서도 어색할 수 있음 → `"Changed to {combo}"`).
   - `capture.rs:405`: `format!("‘{exe}’는 창 단위로만 찾을게요")`.
   - `tauri-plugin-screen-ink/src/lib.rs:1149,1167`: `format!("화면 기록에서 {APP_NAME}을 켜 주세요")` (`ShookShok`으로 바뀌면 조사 제거).
   - `settings.rs:199,205,315`: `‘{exe}’는 …`, `‘{c}’를 …`.
   - `inkEngine.js:213-217,866,1579,1582`: `SHAPE_TO`(`직선으로` 등) + `바꿀까요? · Enter`, `{notice} · {undo}로 원래 획` → 영어는 `draw.shape.suggest.{line,circle,ellipse,rect,arrow}` 완전 문장(`"Convert to line? · Enter"`) 및 `"{notice} · {undo} to revert"`로 분리.

### 4.3 녹화 종료 사유 접두 결합 (`crates/ink-app/src/record.rs:331-358, 405-445`)
현재 `finish_recording(reason: Option<&str>)`가 한국어 어순에 맞춰 문장 앞에 사유·장소를 이어 붙인다.
- `prefix` 6종: `""`, `"화면이 잠겨 "`, `"절전에 들어가 "`, `"화면 구성이 바뀌어 "`, `"창이 닫혀 "`, `"녹화가 멈춰 "`
- `place` 2종: `""`, `"사진 폴더에 "`
- 조합 결과: `"{prefix}{place}녹화를 저장했어요"`, `"{prefix}녹화를 마무리하지 못했어요"`, `"{prefix}녹화를 옮기지 못했어요"`

영어에서는 `"Screen locked Saved to Pictures recording"`처럼 접두 결합이 불가능하다. **권장 리팩터링**: `reason`을 문자열 조각이 아니라 `enum StopReason { User, Lock, Suspend, DisplayChanged, WindowClosed, Stalled }`로 받고,
- **제목(`text`)**: `"Recording saved"` / `"Saved to Pictures"` / `"Couldn't finish recording"` / `"Couldn't move recording"` (짧게 고정 → 섬 폭 절약!)
- **부가 설명(`detail`)**: 사유가 있을 때 `"Screen locked · filename.mp4"` 형태로 합치기 (또는 5종 사유 키로 깔끔히 조합).

### 4.4 지연 캡처 카운트다운 어순 (`crates/ink-app/src/quick.rs:149` + `Island.svelte:247-248`)
`Island.svelte:247-248`이 `<span class="ink-island-countdown">{countdown}</span>` 뒤에 `<strong>{text}</strong>`를 고정 배치한다. 한국어는 `[3]` + `초 뒤 범위 캡처`로 자연스럽지만 영어는 `[3] s · Region capture`처럼 아주 짧게 쓰거나, `Island.svelte`가 `countdown`을 `t('island.delay.countdown', { n: countdown })` 안에서 배치하도록 바꿔야 한다.

### 4.5 문화권 고유 표현 · 브랜드 표기 · `catalog-v1.json`
- **`"한글·PPT에 {paste}"`** (`capture.rs:539`, `draw.rs:916`, `window_capture.rs:198`, `fake.js:366,472`): `한글(HWP)`은 한국어권 전용 소프트웨어다. 영어에서는 `"Paste with {paste}"` 또는 `"Ready to paste · {paste}"`로 일반화해야 한다.
- **앱 이름·제작자**: `ko` = `"슉쇽"`, `en` = `"ShookShok"` (`docs/brand/README.md`, `microsoft-store-listing.md:23`). 제작자 `"찰떡쌤"`(`SettingsWindow.svelte:226`, `MakerCredit.svelte:28,30`, `msix.config.json:5`)의 영문 표기(로마자 음차 vs 원문 유지)는 구현 전 확정 필요.
- **보조 창 제목 브랜드 불일치(정리 필요)**: 현재 코드에 `"슉쇽"`(`lib.rs:46`), `"Shookshok"`(`tauri.conf.json:3`), `"ShookShok island/overlay/pin"`(`lib.rs:450`, `overlay.rs:192`, `pins.rs:99`), `"shookshok record frame"`(`rec_frame.rs:29`), 옛 이름 `"screen-ink zoom"`(`zoom_hud.rs:27,53`)이 혼재한다.
- **`docs/shortcuts/catalog-v1.json`의 `refs[].what` (258건, 고유 233개)**: 단축키 충돌 시 `Analysis::reason()` → `ShortcutRow.reason`에 직접 노출된다. 그중 `hwp`(10건), `icz`(아이캔 줌잇 5건), `ime`(3건)는 한국어권 앱이지만, `win`(62)·`winapp`(60)·`mac`(40)·`macapp`(28)·`pt`(16)·`zi`(14)·`ppt`(11)·`snip`(6)·`ptaot`(3) 등 240건은 글로벌 앱/OS 기능 설명이므로 영어 번역이 필요하다.

---

## 5. macOS vs Windows 문구 차이 및 발견된 OS 하드코딩 버그

### 5.1 플랫폼 전용 문구 분포
| 구분 | macOS 전용 | Windows 전용 |
|---|---|---|
| 수식키 표기 | `⌃` `⌥` `⇧` `⌘` (`spoken`, `keycaps`) | `Ctrl` `Alt` `Shift` `Win` |
| 권한 / 시작 | 화면 기록·손쉬운 사용 권한 안내·버튼·칩 13건 (`FirstRunSheet.svelte:136-139`, `service.rs:376-383,918`, `tauri-plugin-screen-ink/src/lib.rs:1149-1185`) | MSIX `StartupTask` 정책 안내 3건 (`package.rs:270-282`), 저장 위치 `MSIX 패키지` (`package.rs:135`, `settingsMeta.js:121`) |
| 단축키 규칙 | `fn` 표준 기능 키 안내 2건 (`FirstRunSheet.svelte:175`, `shortcuts.rs:590`), `⌥ + 글자` 특수문자 경고 1건 (`shortcuts.rs:623`), `catalog-v1.json` mac refs 68건 | `topmost` 창 항상 위 7건 (`GeneralPage.svelte:141-171`, `TrayPopoverScreen.svelte:146,164`, `catalog-v1.json`), `PowerToys` 모듈 14건 (`powertoys.rs:11-25,65`), PowerToys 항상 위 해제 힌트 (`settingsMeta.js:138`), `catalog-v1.json` win refs 190건 |
| OS 번들 / 스토어 | `Info.plist:7` (`CFBundleDisplayName` = `슉쇽`) — **주의: 현재 `NSScreenCaptureUsageDescription`·`NSAccessibilityUsageDescription`·`NSMicrophoneUsageDescription` 및 `ko.lproj`/`en.lproj` `InfoPlist.strings`가 없음!** | `msix.config.json:3-5`, `AppxManifest.template.xml:36-37` (`ko-KR`/`en-US` 선언은 있으나 단일 한국어 문자열 주입 중 → PRI 리소스 필요), `microsoft-store-listing.md` |

### 5.2 조사 중 발견한 OS별 단축키 하드코딩 버그 (영어화 시 함께 수정 권장)
1. `packages/ink-ui/src/lib/models/settingsMeta.js:106` — `'0.5초 · Ctrl+Z = 원래 획'` (`platform === 'macos'`일 때도 `⌘Z`가 아닌 `Ctrl+Z`로 표시됨).
2. `packages/ink-ui/src/lib/screens/DrawZoomPage.svelte:85` 및 `packages/ink-ui/src/lib/components/AnnotateOptions.svelte:99,117` — `Ctrl+휠` 하드코딩 (macOS는 `⌘+휠`).
3. `packages/ink-ui/src/lib/screens/DrawZoomPage.svelte:170` — `sub="Alt+Shift+1로 다시 열기"` 하드코딩 (macOS 기본값 `⌥⇧1` 및 사용자가 바꾼 `restore` 단축키 미반영).
4. `packages/ink-ui/src/lib/screens/CapturePage.svelte:212` — `sub="Alt를 누르는 동안 끔"` 하드코딩 (macOS는 `Option`/`⌥`).
5. `packages/ink-ui/src/lib/screens/CapturePage.svelte:236` & `crates/ink-domain/src/settings.rs:199` — `placeholder="app.exe"` 및 `‘{exe}’는 실행 파일 이름이 아니에요(예: hwp.exe)`가 Windows 전용 기준(`.exe` 검증)으로 고정돼 있음.

---

## 6. 영어 길이 팽창으로 UI가 깨질 위험 구간

영어는 한국어 대비 평균 **1.5×~2.3×** 길다(예: `"복사했어요"` 5자 → `"Copied"` 6자 / `"모서리 자석"` 6자 → `"Edge snapping"` 13자 / `"요소까지 잡기 · 손쉬운 사용 켜기"` 18자 → `"Detect elements · Enable Accessibility"` 38자). 실제 CSS 고정 폭과 `white-space: nowrap`을 대조한 상위 5대 위험 구간은 다음과 같다.

| 순위 | 화면 / 컴포넌트 | 레이아웃 제약 (실제 CSS·코드 근거) | 깨질 위험이 큰 문구 예시 | 완화 방안 |
|---:|---|---|---|---|
| **1** | **섬 (`Island.svelte` / `islandQueue.js`)** | `max-width: 520px; white-space: nowrap;` (`Island.svelte:494,504`). 결과 섬은 썸네일(74px)+제목+메타+버튼 2개+HEX 칩이 한 줄에 몰림. 경로 축약 `formatCapturePath`가 `maxChars = 20` (`islandQueue.js:225`) | `화면 › 창 › 표 › 2행`(13자) → `Screen › Window › Table › Row`(29자, `… › Row`로 과다 절단) / `요소까지 잡기 · 손쉬운 사용 켜기` / `판서는 영상에 함께 · 이 알약은 안 찍혀요` / 결과 섬 `저장`+`판서로`+`#FF9F0A 복사` | 영어 짧고 간결한 카피(동사 1~2어: `Save`, `Draw`, `Copy #FF9F0A`), `formatCapturePath` `maxChars`를 언어별 또는 픽셀 폭 기준으로 확대(예: `en` 28~32자), 결과 섬 `max-width` 560~600px 검토 또는 보조 칩 줄임 |
| **2** | **트레이 팝오버 (`TrayPopover.svelte` / `TrayPopoverScreen.svelte`)** | 고정 `width: 288px` (`TrayPopover.svelte:116`, R2-UI-TRAY 목표는 `248~260pt`로 더 좁음), 모든 행·헤더·푸터 `white-space: nowrap` (`TrayPopoverScreen.svelte:230-344`) | 헤더 `ShookShok` + `Recording 00:42` / `Shortcuts paused` / `3 key conflicts`, 고정 창 행 `Pinned captures` + 카운트 + `Close all`, 푸터 `Settings… Ctrl+,` + `Quit` | 팝오버 상태 라벨을 2단어 이내로 압축(`Paused`, `3 conflicts`, `Rec 00:42`, `Release all` → `Unpin all`), 메뉴 라벨 좌측에 `min-width: 0; overflow: hidden; text-overflow: ellipsis` 안전장치 추가 |
| **3** | **단축키 한 장 (`CheatSheet.svelte`)** | `width: min(820px, 100vw - 32px)`, 3단 균등 그리드(`repeat(3, minmax(0, 1fr))` ≈ 열당 245px), 행 높이 26px 단행 `ellipsis` (`CheatSheet.svelte:125,167,221-224`) | `창 ↔ 요소 전환` (`Window ↔ Element`), `복사하고 끝내기` (`Copy & exit`), `아래 앱 스크롤` + `Shift` `Scroll`, `확대 중 이동` + `Right-click`, `멈추면 도형` + `0.5s` | 2단어 내 짧은 명사/동사형 사용(`Detect mode`, `Copy & close`, `Scroll behind`, `Pan zoomed`), 키캡/노트 약어(`R-click`, `2× click`, `Arrows`) 적용 |
| **4** | **설정 창 `Row` + `Segmented` + 사이드바 (`SettingsWindow.svelte`, `Row.svelte`, `CapturePage`, `DrawZoomPage`, `GeneralPage`)** | 창 `900×600`, 사이드바 `196px` `nowrap` (`SettingsWindow.svelte:259,273,306,328`), `Row` 단행 `nowrap` (`Row.svelte:185,208`), `Segmented` `nowrap` (`Segmented.svelte:85`) | 사이드바 `Capture & recording`(~155px, 아슬아슬), `zoom.follow` 3분할(`At cursor` / `Smooth center` / `Instant`), `draw.shapeApply` 2분할(`Ask first` / `Auto`), `capture.excluded` 칩 `느림 · 창 단위 자동` + 입력칸 + `Add` | Segmented 옵션 1~2단어 제한(`Follow cursor` / `Center` / `Instant`, `Ask` / `Auto`, `System` / `Light` / `Dark`), `CapturePage` 제외 앱 행은 `flex-wrap: wrap` 허용 검토 |
| **5** | **배너 · 첫 실행 시트 · 위치 지도 (`Banner.svelte`, `FirstRunSheet.svelte`, `MiniMap.svelte`)** | `FirstRunSheet` `width: min(520px, 100vw - 32px)` (`FirstRunSheet.svelte:218`), 배너 텍스트 `white-space: nowrap; text-overflow: ellipsis` + 버튼 2개 (`Banner.svelte:83-85`, `FirstRunSheet.svelte:356-358`), `MiniMap` `112×70` / `96×60` (`MiniMap.svelte:50,59`) | 충돌 배너 `Snipaste is already using F1 & F3 (4 keys)` + `How to turn off` + `Change key`, `MiniMap` 하단 `Display 1` + `×2.0`, `FirstRunSheet` `F1·F2·F3` 안내 문구 | 배너에서 2줄 줄바꿈 허용(`white-space: normal; line-height: 1.4`) 또는 버튼 축약(`Disable guide` / `Change`), `MiniMap`은 `Display 1` 또는 `Screen 1` 유지 |

---

## 7. 구현 제안 (가)~(마)

### (가) 카탈로그 위치 및 파일 구조
- **권장안 — 현 선반영 경로 `packages/ink-ui/src/lib/i18n/{ko,en}.json`을 단일 원본(SSOT)으로 유지**:
  - 이미 `crates/ink-domain/src/i18n.rs:11-12`가 `include_str!("../../../packages/ink-ui/src/lib/i18n/{ko,en}.json")`로 직접 컴파일 포함하고, JS/Svelte(`packages/ink-ui/src/lib/i18n/core.js`)도 같은 JSON을 정적 import 한다.
  - PRD §2에 언급된 루트 `locales/` 대비 장점: `packages/ink-ui`를 독립 패키지(미리보기·Vitest·Vite 번들)로 빌드할 때 패키지 바깥(`../../../locales`) import로 인한 Vite FS 보안 제한이나 패키징 누락을 피할 수 있다.
  - **형식**: 평면(flat) 점 표기 JSON (`"domain.section.key": "문구"`). 중첩 객체보다 Rust `BTreeMap<String, String>` 파싱·diff 검토·키 검색(`grep`)이 단순하다.
- **`docs/shortcuts/catalog-v1.json`의 `refs[].what` 258건 분리**:
  - `slots`(57)와 `sources`(12)는 핵심 `ko.json`/`en.json`에 `shortcut.slot.<id>`, `shortcut.source.<id>`로 합쳐 두는 편이 일관된다(UI `TrayPopoverScreen`, `CheatSheet`, `FirstRunSheet`, Rust `shortcuts.rs`가 공유).
  - 반면 충돌 상세 설명 `refs[].what`(258건, 고유 233개)은 `docs/shortcuts/catalog-en-v1.json` 같은 별도 매핑 파일(예: `what_ko -> what_en` 또는 `(src, platform, combo) -> what_en`)로 두고 `ink-domain/src/shortcuts.rs`가 `include_str!`로 읽어 `Catalog::analyze(..., lang)`에서 치환하는 것이 핵심 UI 카탈로그의 비대화를 막아 준다.
- **OS 정적 리소스(런타임 JSON 밖)**:
  - **macOS**: `apps/screen-ink/src-tauri/resources/{ko,en}.lproj/InfoPlist.strings`에 `CFBundleDisplayName` (`슉쇽` / `ShookShok`)과 **현재 누락된 권한 설명 3종**(`NSScreenCaptureUsageDescription`, `NSAccessibilityUsageDescription`, `NSMicrophoneUsageDescription`)을 추가해 `tauri.macos.conf.json` 번들에 포함한다.
  - **Windows MSIX**: `AppxManifest.template.xml`의 `DisplayName`/`Description`을 `ms-resource:AppDisplayName`/`ms-resource:AppDescription`으로 바꾸고 `Strings/{ko-KR,en-US}/Resources.resw`를 `makepri.exe`로 묶도록 `scripts/package-msix.ps1`을 확장한다.

### (나) Rust가 만든 섬·오류 문구 번역 방식 비교
| 기준 | 방식 A: Rust가 `key + args`를 보내고 UI가 번역 | 방식 B: Rust가 현재 `Lang`으로 번역해 완성 문자열 전송 *(현재 선반영 방식)* |
|---|---|---|
| IPC·직렬화 계약 | `IslandEvent`, `ShortcutRow`, `UpdateResult`, `ImportPreview` 스키마 전면 변경 필요 | **기존 문자열 계약 그대로 유지** (변경 최소) |
| 트레이·OS 대화상자·핀 메뉴 | UI가 없으므로 어차피 Rust 번역기가 또 필요(이중화) | Rust `ink-domain::i18n::tr/trf` **한 경로로 통일** |
| 언어 전환 직후 화면 반영 | 이미 떠 있는 섬도 재렌더 즉시 바뀜 | 일회성 결과 섬은 1.2초 뒤 닫히므로 체감 차이 미미. 단, `InkState`에 실린 `shortcuts`/`notices`·활성 모드 섬은 **언어 변경 시 재발행 필요** |
| `fake.js` 일치성 | `fake.js`도 키+인자를 보내야 함 | `fake.js`가 같은 `t(key, params)`를 쓰면 동일 동작 |
| Rust 단위 테스트 | 키·인자 단언 가능 | `Lang::Ko`/`Lang::En` 문자열 직접 검증 가능 |

- **권장 결정: 방식 B(Rust 번역) 중심 + 상태 재발행 보강(하이브리드)**
  - 이미 `crates/ink-domain/src/i18n.rs`와 `service.rs:632-671`이 방식 B로 잘 자리 잡았고, 트레이 툴팁·핀 우클릭 메뉴·OS 파일 대화상자·설정 오류(`SettingError`/`NamingError`)까지 단일 엔진으로 처리할 수 있다.
  - 보완 2가지:
    1. `general.lang`이 바뀌면 `InkService::update_setting`이 `emit_state()`를 호출할 때 `ShortcutRow.label`/`reason`, `autostart_notice`, `notices`가 새 `self.lang()`으로 다시 만들어지도록 하고(저장 시점 문자열 캐시 금지, `AutostartNotice`와 `Loaded.notice`는 enum 코드로 보관 후 `state()` 생성 시 번역!), 열려 있는 모드 섬(`capture`/`draw`/`zoom`/`record`)이 있으면 한 번 재발행한다.
    2. `recog.rs`의 `Control::label(lang)` / `NodeKind::label(lang)`도 `Lang`을 인자로 받도록 시그니처만 확장하면 경로 빵부스러기까지 즉시 영어로 나온다.

### (다) 설정을 바꾸면 바로 바뀌게 (`R-SET-0`)
현재 구현 점검 결과, **중요한 빈틈이 하나 확인되었다**: `syncLang(state)`가 `SettingsWindow.svelte:73,79`에만 연결돼 있어 나머지 7개 WebView 창(`TrayPopoverScreen`, `IslandHost`, `CaptureOverlay`, `DrawOverlay`, `PinView`, `RecordFrame`, `ZoomHud`)은 `general.lang`을 바꿔도 `locale.lang`이 갱신되지 않는다.

즉시 반영을 위한 4단계 체크리스트:
1. **전 창 공통 동기화**: `apps/screen-ink/src/App.svelte` 최상단 `$effect`에서 `client.getState()`와 `client.subscribe('state')`로 `syncLang(state)`를 1회 공통 호출한다. `syncLang` 내부에서 `document.documentElement.lang = lang`과 `document.title = t('app.name')`도 함께 갱신한다.
2. **Svelte 5 룬 반응성 보장**: 컴포넌트 `<script>` 최상단 상수로 묶여 있는 한글 배열/객체를 키 기반 또는 `$derived`로 바꾼다.
   - 대상: `CapturePage.svelte` `MIC_DEVICES`, `DrawZoomPage.svelte` `PEN_COLORS`, `FirstRunSheet.svelte` `THREE`, `settingsMeta.js` `SETTING_LABELS`/`ENUM_LABELS`(함수로 래핑해 `t()` 호출), `captureToolbar.js` `ACTIONS`/`TOOLS`, `cheatSheet.js` `GLOBAL_FALLBACKS`, `drawUi.js` `PEN_SWATCHES`.
3. **Rust 측 언어 변경 부수효과 (`crates/tauri-plugin-screen-ink/src/lib.rs:700` `update_setting`)**:
   - `path == "general.lang"` 성공 시:
     - `TauriNotifier`의 `tray_mode` 캐시를 무효화하거나 캐시 키에 `Lang`을 포함시켜 `tray.set_tooltip(Some(tooltip(state, lang)))` 즉시 갱신.
     - 열려 있는 네이티브 창(`settings`, `popover` 등)의 제목을 `w.set_title(...)`로 즉시 갱신.
     - `pins.rs` 우클릭 메뉴는 우클릭할 때마다 `Menu::with_items`로 생성되므로 현재 `lang`만 전달하면 즉시 반영.
4. **첫 실행 OS 언어 기본값 (`R2-I18N-1`)**: 설정 파일이 없는 첫 실행 시 OS 언어(`ko*` → `ko`, 그 외 → `en`)를 `general.lang` 초기값으로 주입하되, 한 번 저장된 사용자 선택은 덮어쓰지 않는다.

### (라) 빠진 키 폴백(Fallback) 규칙
- **3단계 폴백**: `catalog[lang][key]` *(존재하고 `trim()` 비어 있지 않을 때)* → `catalog.ko[key]` → `key` 원문 그대로.
- `crates/ink-domain/src/i18n.rs:59-61`과 `packages/ink-ui/src/lib/i18n/core.js`가 이미 이 순서를 구현하고 있다. 여기에 **빈 문자열(`""`) 방어**와 **개발 빌드(`#[cfg(debug_assertions)]` / `import.meta.env.DEV`) 1회 경고 로그**를 추가해 누락 키를 개발 중 즉시 눈에 띄게 만든다.
- **복수형 도우미**: 무거운 ICU 대신 `ko`/`en` 2언어 전용 `trp(lang, base_key, n, args)` / `tp(baseKey, n, params)`를 도입한다. 규칙: `en`은 `n === 1 ? "${baseKey}.one" : "${baseKey}.other"`, `ko`는 `${baseKey}.other`(없으면 `${baseKey}`) 조회.

### (마) 시험 · 정적 검사 방법
1. **카탈로그 정합성 단위 시험 (이미 골격 존재 → 확장)**:
   - Rust (`crates/ink-domain/src/i18n.rs`) + Node (`packages/ink-ui/src/lib/i18n/core.test.js`):
     - `keys(en.json) == keys(ko.json)` (빠진 키·남는 키 즉시 실패).
     - 모든 키에 대해 `placeholders(en[k]) == placeholders(ko[k])` (`{n}`, `{app}` 등 이름·개수 일치).
     - 빈 문자열/공백 문자열 금지.
     - `.one` 키가 있으면 반드시 짝 `.other` 키 존재 확인.
     - `catalog-en-v1.json` 분리 시 `catalog-v1.json`의 모든 고유 `refs[].what`이 매핑되었는지 검증.
2. **코드↔카탈로그 사용 검증 스크립트 (`npm run check:i18n`, 경량 Node 스크립트)**:
   - `.svelte`/`.js`/`.rs`에서 `t('...')`, `tp('...')`, `self.t("...")`, `self.tf("...")`, `tr(..., "...")` 호출의 정적 키를 추출해:
     - **없는 키 참조** (오타·미등록 키) → CI 실패.
     - **미사용(남는) 키** (동적 접두어 키 `shortcut.slot.*`, `recog.control.*` 등은 패턴 allowlist) → 경고 또는 실패.
3. **잔여 하드코딩 한글 검사**:
   - 주석·테스트·`ko.json`·명시적 allowlist(`한국어`, `한/영`, `한자`, `naming.rs`의 `"앱"` 별칭 등)를 제외한 프로덕션 소스에서 `[가-힣]` 문자열 리터럴을 탐지해 새로 추가되는 기능의 영어 누락을 방지.
4. **영어 UI 넘침 스모크 시험**:
   - `general.lang = 'en'` 상태에서 핵심 5대 위험 화면(섬 520px, 트레이 팝오버 288px/260px, 단축키 한 장 820px, 설정 창 900×600 각 탭, 첫 실행 시트 520px)을 미리보기/Playwright로 열어 `el.scrollWidth <= el.clientWidth + 1` 단언(특히 `white-space: nowrap` 버튼·탭·세그먼트).
