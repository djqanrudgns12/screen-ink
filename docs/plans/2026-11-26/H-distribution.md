# H. Windows 배포 · 스토어 · 서명 · 자동 업데이트 구현 시안 (`D2`·`D3`·`D5`·`D8`·`D9`·`D10`·`D11`)

대상 항목(`progress.json`): `D2` MSIX 포장 파이프라인 · `D3` 패키지 모드 대응 · `D5` 스토어 인증 제출·통과 · `D8` 서명한 MSIX 한 파일 배포판 · `D9` `.appinstaller` 자동 업데이트 호스팅·릴리스 스크립트 · `D10` 업데이트 팝업(클릭 한 번 몇 초 안 설치·자동 재시작) · `D11` SignPath Foundation 신청.
연계 진행 중 항목: `D1`(개인 개발자 계정 신분증 인증 — 사용자 진행), `D4`(스토어 등록 자료 최종 스크린숏·로고), `D6`(`winget` msstore 확인).
기준 문서: [docs/prd/11-distribution.md](../../prd/11-distribution.md)의 2026-11-26 결정을 그대로 따른다.

---

## 1. 목표 · 범위 (요구사항 ID · 결정)

- **기본 채널**: Microsoft Store (MSIX, 스토어 서명, 비용 0원, 설치 경고 0, 자동 업데이트).
- **직접 파일 공유(스토어 차단 PC)**: SignPath Foundation 무료 서명(조건: 공개 MIT 저장소 `D7` 완료 + GitHub Actions 빌드) → 서명 MSIX + `https://ink.ssamuniverse.com/screen-ink.appinstaller` (`D12` 완료).
- **네트워크 0 원칙 유지 (`R-NET-1` 개정·`R-NET-2`)**: 앱 실행 파일에는 HTTP 클라이언트 없음. 업데이트 확인·다운로드·설치는 Windows OS API(`StoreContext` / `PackageManager` + `.appinstaller`)를 호출해 OS가 통신.
- **업데이트 UX (`11 §7.2`, 사용자 결정)**: 배너가 아니라 **모니터 가운데 팝업(`Sheet`)** → `[지금 업데이트]`(기본) · `[오늘은 그만]`, 클릭 한 번에 몇 초 안에 설치·자동 재시작. 모드·전체 화면 앱·보안 데스크톱 중에는 미룸.

---

## 2. 정확한 형식

### 2.1 `D2` MSIX 포장 파이프라인 (`packaging/msix/`)

Tauri는 MSIX를 직접 만들지 않으므로, `cargo tauri build --no-bundle`이 만든 `screen-ink.exe`를 Windows SDK `makeappx.exe`로 감싸는 파이프라인을 만든다.

```
packaging/msix/
├─ AppxManifest.template.xml   # 스토어판 / 직접배포판 공용 템플릿 (Publisher·Version 치환)
├─ assets/                     # StoreLogo(50), Square44x44Logo, Square150x150Logo, Wide310x150Logo + scale-100/125/150/200/400
└─ build-msix.ps1              # -Channel store|direct -Version 0.2.0.0 -CertThumbprint ...
```

**`AppxManifest.template.xml` 핵심 선언**:
```xml
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
         xmlns:desktop="http://schemas.microsoft.com/appx/manifest/desktop/windows10"
         xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
         IgnorableNamespaces="uap desktop rescap">
  <Identity Name="{{PACKAGE_NAME}}" Publisher="{{PUBLISHER}}" Version="{{VERSION}}" ProcessorArchitecture="x64" />
  <Properties>
    <DisplayName>{{DISPLAY_NAME}}</DisplayName>
    <PublisherDisplayName>찰떡쌤</PublisherDisplayName>
    <Logo>Assets\StoreLogo.png</Logo>
  </Properties>
  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.19041.0" MaxVersionTested="10.0.26100.0" />
  </Dependencies>
  <Applications>
    <Application Id="ScreenInk" Executable="screen-ink.exe" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements DisplayName="{{DISPLAY_NAME}}" Description="발표자를 위한 화면 캡처·판서·확대 도구"
                          BackgroundColor="#141416" Square150x150Logo="Assets\Square150x150Logo.png"
                          Square44x44Logo="Assets\Square44x44Logo.png" />
      <Extensions>
        <desktop:Extension Category="windows.startupTask" Executable="screen-ink.exe" EntryPoint="Windows.FullTrustApplication">
          <desktop:StartupTask TaskId="ScreenInkStartup" Enabled="true" DisplayName="{{DISPLAY_NAME}}" />
        </desktop:Extension>
      </Extensions>
    </Application>
  </Applications>
  <Capabilities>
    <rescap:Capability Name="runFullTrust" />
  </Capabilities>
</Package>
```
- **채널별 치환**:
  - `store`: `Publisher` = Partner Center 발급 `CN=...`, 서명 없이 `.msix` 생성(스토어가 서명).
  - `direct`: `Publisher` = SignPath Foundation 인증서 주체(글자 그대로 일치, `11 §10.3`), GitHub Actions에서 SignPath 서명 요청.

### 2.2 `D3` 패키지 모드(MSIX) 런타임 대응

- **감지 모듈**: `crates/ink-platform-windows/src/package.rs` (신규).
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub enum InstallChannel {
      Unpackaged,      // NSIS · 포터블 ZIP
      MsixStore,       // 패키지 모드 + AppInstaller 없음 (Microsoft Store)
      MsixDirect,      // 패키지 모드 + GetAppInstallerInfo() 있음 (.appinstaller 직접 배포)
  }
  /// `GetCurrentPackageFullName` 반환이 `APPMODEL_ERROR_NO_PACKAGE`가 아니면 패키지 모드.
  pub fn detect_channel() -> InstallChannel;
  ```
- **3가지 동작 분기 (`11 §5`)**:
  1. **로그인 자동 실행 (`general.autostart`)**:
     - `Unpackaged`: 기존 `tauri-plugin-autostart` (Run 레지스트리).
     - `MsixStore | MsixDirect`: WinRT `StartupTask::GetAsync("ScreenInkStartup")` → `RequestEnableAsync()` / `Disable()`. 사용자가 Windows 설정에서 끈 상태(`DisabledByUser`)이면 설정 창에 `"Windows 시작 앱 설정에서 켜 주세요"` 안내.
  2. **포터블 표시 파일 (`screen-ink.portable`)**:
     - `ink-store::resolve_dir`에 `is_packaged: bool` 인자 추가 → 패키지 모드에서는 설치 폴더 읽기 전용이므로 표시 파일을 **무시**하고 가상화된 `%APPDATA%\screen-ink` 사용.
  3. **설정 위치 표기 (`S05 일반 > 설정 위치`)**:
     - `"MSIX 패키지"` / `"설치"` / `"포터블"` 구분 표시.

### 2.3 `D8` 서명 MSIX 직접 배포판 & WebView2 부재 안내

- **WebView2 검사**: `apps/screen-ink/src-tauri/src/main.rs` 진입 최상단(`#[cfg(windows)]`)에서 `GetAvailableCoreWebView2BrowserVersionString` 호출.
  - 실패(오래된 Windows 10 등) 시 Tauri WebView 생성 전에 네이티브 `MessageBoxW`로 안내:
    `"Microsoft Edge WebView2 런타임이 필요해요. 확인을 누르면 안내 페이지로 열려요."` (또는 설치 안내 문구 — 앱 자체는 다운로드 통신 금지).
- **내려받은 파일 경고 확인**: SignPath 서명 완료 후 다른 PC에서 브라우저로 내려받아 더블클릭 → 앱 설치 관리자 화면에 게시자 이름·경고 유무 실측 기록(`11 §6 [확인 필요]`).

### 2.4 `D9` `.appinstaller` 호스팅 · 릴리스 스크립트

- **고정 주소 (`D12` 완료)**: `https://ink.ssamuniverse.com/screen-ink.appinstaller` (원본 `site/screen-ink.appinstaller` → `gh-pages` 발행).
- **릴리스 스크립트**: `scripts/release-appinstaller.ps1`
  - 입력: `-Version 0.2.0.0 -Tag v0.2.0 -Publisher "<SignPath 주체>"`
  - 생성 파일: `site/screen-ink.appinstaller` (2021 스키마, `11 §7.1` XML 형식 그대로, `Uri`는 `https://github.com/djqanrudgns12/screen-ink/releases/download/<Tag>/screen-ink_<Version>_x64.msix` — 리디렉션 주소 대신 정확한 태그 URL로 리디렉션 미지원 위험 원천 차단).
  - `UpdateSettings`: `<OnLaunch HoursBetweenUpdateChecks="0" ShowPrompt="false" UpdateBlocksActivation="false" />` + `<AutomaticBackgroundTask />`.

### 2.5 `D10` 업데이트 팝업 — 데이터 · 포트 · OS API 흐름

#### 포트 (`Updater`, `crates/ink-app/src/ports.rs`)

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadyUpdate {
    pub version: String,     // 예: "0.2.0"
    pub channel: &'static str, // "store" | "direct"
}

pub trait Updater: Send {
    /// OS API로 업데이트 확인 + 백그라운드 사전 다운로드 완료 여부 확인.
    /// 이미 준비 완료된 새 버전이 있으면 Some(ReadyUpdate). 실패 시 Err(무시, 앱 동작 영향 0).
    fn check_and_stage(&mut self) -> Result<Option<ReadyUpdate>, String>;
    /// 클릭 한 번 설치 + RegisterApplicationRestart 자동 재시작.
    fn install_and_restart(&mut self) -> Result<(), String>;
}
```
- 1차 비패키지 빌드(NSIS/포터블)·맥: `NoopUpdater` (`Ok(None)`).
- MSIX 스토어판(`MsixStore`): `Windows::Services::Store::StoreContext::GetDefault()` → `GetAppAndOptionalStorePackageUpdatesAsync` → `TrySilentDownloadStorePackageUpdatesAsync` (미리 받기) → 클릭 시 `RegisterApplicationRestart` + `TrySilentDownloadAndInstallStorePackageUpdatesAsync`.
- MSIX 직접 배포판(`MsixDirect`): `Package::Current().CheckUpdateAvailabilityAsync()` + `PackageManager::StagePackageByUriAsync` (또는 `.appinstaller` 백그라운드 스테이징) → 클릭 시 `RegisterApplicationRestart` + `AddPackageByAppInstallerFileAsync(..., ForceTargetAppShutdown)`. `[확인 필요]` 두 WinRT API의 실제 권한·서명 환경 동작은 `D2` 로컬 테스트 서명 MSIX로 우선 검증한다.

#### 연기 조건 & 화면 (`UpdateSheet.svelte`)

- **확인 시점 (`11 §7.2`)**: 시작 1분 뒤 → 6시간마다 → 트레이 팝오버 열 때.
- **미루는 조건 (`should_defer_update` 순수 함수, `crates/ink-domain/src/update.rs`)**:
  1. 현재 모드가 `Idle`이 아니거나 녹화 활성 중(`mode != Mode::Idle || recording.active`)
  2. 전체 화면 앱이 앞에 있음(`SHQueryUserNotificationState`가 `QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE`)
  3. 보안 데스크톱(`secure_desktop == true`)
  4. 사용자가 `"오늘은 그만"`을 눌렀고 오늘 지역 날짜(`settings.update.snoozeDate`)와 같음
  - 위 조건이 풀리는 순간(모드 종료·발표 종료 감지 시) 즉시 팝업 표시.
- **팝업 UI (`packages/ink-ui/src/lib/screens/UpdateSheet.svelte`, 창 `ink-update`, 커서 모니터 가운데)**:
  - 디자인 시스템 `Sheet` 모양, 앱 아이콘(40px) + 제목 `"새 버전이 준비됐어요"` + 보조 `"v0.2.0 · 누르면 몇 초 만에 바뀌고 다시 켜져요"`.
  - 단추: `[지금 업데이트]`(주황/반전 기본, `Enter`) · `[오늘은 그만]`(`Esc`).
  - 클릭 후 진행 중 섬: `"업데이트 중 — 곧 다시 켜져요"`.
- **`InkClient` 확장**: `InkState.readyUpdate?: ReadyUpdate`, 액션 `'update.install'` / `'update.snoozeToday'`.
- **문서 동기화**: `D10` 구현 PR에서 `docs/PRD.md`의 `R-NET-1` 문구를 `11 §7.2` 결정과 일치하게 함께 갱신(이 시안 단계에서는 PRD 미수정).

### 2.6 `D11` SignPath Foundation 신청 & `D5` 스토어 인증

- **`D11` 준비물 (`11 §10.3`)**:
  1. 루트 `README.md`에 `## Code Signing Policy` 절 + 필수 문구 `"Free code signing provided by SignPath.io, certificate by SignPath Foundation"` 추가.
  2. `.github/workflows/release-msix.yml`: 태그 `v*` 푸시 시 `windows-latest`에서 `npm ci` → `cargo tauri build --no-bundle` → `build-msix.ps1 -Channel direct` → `signpath/github-action-submit-signing-request`로 서명된 `.msix` 생성 → GitHub Release 업로드.
  3. GitHub 계정 2FA 활성화(사용자) → SignPath Foundation 신청 제출.
- **`D5` 스토어 인증 절차**:
  - `D1` 계정 승인 → 앱 이름 예약 → `docs/store/microsoft-store-listing.md`(`D4`)에 실기 스크린숏·최종 아이콘 반영 → `build-msix.ps1 -Channel store`로 무서명 `.msix` 생성 → WACK(Windows App Certification Kit) 로컬 통과 → Partner Center 제출 → `D6` `winget install <StoreID> --source msstore` 확인.

---

## 3. 알고리즘 · 결정 (대안과 고른 이유)

- **왜 Tauri 내장 업데이터(`tauri-plugin-updater`) 대신 Windows OS API인가?**
  - `tauri-plugin-updater`는 `reqwest`/`winhttp`를 실행 파일에 끌어들여 `R-NET-2`(PE import 검사)를 깨뜨리고, 학교망 SSL 가로채기 환경에서 인증서 오류가 반복된다(Snipaste 95회 실패 사례). Windows 스토어·App Installer는 OS 신뢰 저장소와 Windows Update 채널을 쓰므로 앱 프로세스 네트워크 0 원칙과 완벽히 양립한다.
- **왜 `.appinstaller` URL에 `latest` 리디렉션 대신 버전 고정 URL을 쓰는가?**
  - App Installer가 302 리디렉션을 일부 버전에서 거부하는 문제가 보고되어 있으므로, 릴리스 스크립트가 매 버전 정확한 자산 URL을 써 넣는다.

---

## 4. 시험 · 합격 기준

| 항목 | 시험 / 검증 | 합격 기준 |
|---|---|---|
| `D2` | 로컬 자체 서명 MSIX + WACK(Windows App Certification Kit) | 설치·실행·F1/Alt+1/전역 단축키 정상, WACK 통과 |
| `D3` | Rust 단위 `패키지모드_포터블무시_및_StartupTask분기` + MSIX 설치 실기 | 패키지 모드에서 `screen-ink.portable` 무시, Windows 시작 앱 목록에 `screen-ink` 표시 |
| `D8` | 서명된 `.msix` 다른 PC에서 브라우저 다운로드 후 더블클릭 | 인증서 수동 설치 없이 `[설치]` 진행, SmartScreen/경고 여부 기록 |
| `D9` | `v0.1.0 → v0.2.0` `.appinstaller` 실제 업그레이드 테스트 | 새 버전 자동 감지, `settings.json` 유지 |
| `D10` | Rust 단위 `업데이트_모드중_전체화면_오늘그만_연기판정` + 실기 팝업 클릭 | 모드/전체 화면 중 팝업 0회, 끝나면 즉시 표시, `[지금 업데이트]` 후 10초 이내 재시작 완료 |
| `D5`·`D6` | Partner Center 인증 통과 + `winget install ... --source msstore` | 스토어 공개, 학교 PC 스토어·winget 설치 무경고 |

---

## 5. 위험 · 대응

| 위험 | 발생 신호 | 대응 |
|---|---|---|
| **Partner Center 인증에서 `runFullTrust` / 전역 단축키 / UIA 심사 보류** | 인증 리포트에 사유 기재 요청 | 제출 메모(`docs/store/microsoft-store-listing.md`)에 "화면 캡처·발표 판서 도구로서 전역 단축키·접근성 영역 감지·오프라인 동작" 명시 |
| **SignPath Foundation 심사 거절 또는 MSIX 서명 미지원** | 신청 결과 회신 | `11 §1.1`·`§8` 백업안: Certum 오픈소스 클라우드 서명(€49/년) 전환(사용자 승인 필요) 또는 스토어 단독 채널 유지 |
| **MSIX `PackageManager` 업데이트 API가 트레이 상주 중 재시작 실패** | 설치 후 프로세스가 다시 안 뜸 | `RegisterApplicationRestart("", RESTART_NO_CRASH)` 호출 시점을 `AddPackageByAppInstallerFileAsync` 직전으로 고정, 실패 시 `"업데이트를 적용하려면 앱을 다시 켜 주세요"` 섬 안내 |
| **스토어판과 직접 배포판 동시 설치 충돌** | Publisher가 달라 두 앱이 공존 | 설치 안내 페이지(`site/support`)에 중복 설치 주의 안내 + 단일 인스턴스 뮤텍스 이름으로 이중 실행 방지 |

---

## 6. 작업일 · 난이도 · 먼저 끝나야 할 것

| 항목 | 작업일(남은 일) | 난이도 | 선행 |
|---|---:|---|---|
| `D2` MSIX 포장 파이프라인 | 1.00 (1.00) | 상 | 없음(바로 착수 가능) |
| `D3` 패키지 모드 대응 | 0.50 (0.50) | 중 | `D2` |
| `D11` SignPath Foundation 신청 + 릴리스 CI | 0.50 (0.50) | 중 | `D7`(완료) · `D2` |
| `D8` 서명 MSIX 직접 배포판 | 0.50 (0.50) | 중 | `D2` · `D11` |
| `D9` `.appinstaller` 호스팅·릴리스 스크립트 | 0.50 (0.50) | 중 | `D8` · `D12`(완료) |
| `D10` 업데이트 팝업 + `R-NET-1` 문구 수정 | 1.00 (1.00) | 상 | `D3` · `D9` |
| `D5` 스토어 인증 제출·통과 | 0.50 (0.50) | 중 | **G** + 사용자 `D1` · `D4` |
| *(연계)* `D1`·`D4`·`D6` | 1.00 (0.50) | 하 | 사용자 신분증 인증(`D1`), 실기 스크린숏(`D4`) |
| **묶음 합계** | **5.50일 (5.00일)** | **상** | `D2`·`D3`·`D11`은 병렬, `D5`는 **G** + `D1` 후 |
