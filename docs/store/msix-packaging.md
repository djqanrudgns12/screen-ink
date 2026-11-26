# Windows MSIX 포장 · 패키지 모드 대응 안내 (`D2` · `D3`)

기준 결정: [docs/prd/11-distribution.md](../prd/11-distribution.md) · 구현 시안: [docs/plans/2026-11-26/H-distribution.md](../plans/2026-11-26/H-distribution.md).

## 1. 포장 파일 구성 (`D2`)

- 템플릿·설정: `apps/screen-ink/src-tauri/packaging/msix/`
  - `AppxManifest.template.xml`: 스토어판(`store`)·직접 배포판(`direct`) 공용 매니페스트 템플릿 (`runFullTrust`, `windows.startupTask`, `uap11` WGC 기능 선언 포함).
  - `shookshok.appinstaller.template.xml`: 직접 배포판 2021 스키마 `.appinstaller` 템플릿 (`https://ink.ssamuniverse.com/shookshok.appinstaller`).
  - `msix.config.json`: 앱 표시 이름(`슉쇽 (ShookShok)`), Identity Name(`com.ssamuniverse.shookshok`, 스토어판은 `-PackageName` / `vars.STORE_PACKAGE_NAME`으로 Partner Center 예약값 덮어쓰기 가능), 게시자 표시 이름(`찰떡쌤`), `StartupTask` ID(`ShookshokStartup`), 산출물 접두어(`shookshok`), 실행 파일 후보(`shookshok.exe`·`screen-ink.exe`), `.appinstaller` 주소 단일 원본.
- 아이콘: `apps/screen-ink/src-tauri/icons/`의 기존 PNG(`StoreLogo.png`, `Square44x44Logo.png`, `Square71x71Logo.png`, `Square150x150Logo.png`)를 스테이징 폴더로 **읽어서 복사만** 한다.
- 포장 스크립트: `scripts/package-msix.ps1`
  - 릴리스 실행 파일(`shookshok.exe` 또는 `screen-ink.exe`)을 찾아 실제 파일 이름으로 매니페스트 `Executable`을 채운다.
  - 산출물: `shookshok_<버전>_x64_<채널>.msix`, `shookshok.appinstaller`.

## 2. 로컬 실행 방법

### Windows SDK 요구 사항

`makeappx.exe`(및 시험 서명 시 `signtool.exe`)가 필요하다. 스크립트는 `PATH`와 `C:\Program Files (x86)\Windows Kits\10\bin\<최신>\<arch>\`를 자동 탐색한다.

없을 때 설치 방법:

```powershell
winget install --id Microsoft.WindowsSDK.10.0.26100 -e
```

### 스토어용 무서명 `.msix` 포장 + 검증

```powershell
# npm scripts 사용 (빌드 포함)
npm run msix:store

# 이미 빌드한 release exe로 출력 폴더 지정
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package-msix.ps1 `
  -Channel store -SkipBuild -OutDir D:\dev-cache\screen-ink\out -Verify
```

### 직접 배포용 `.msix` + `.appinstaller` 생성

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package-msix.ps1 `
  -Channel direct -SkipBuild -OutDir D:\dev-cache\screen-ink\out `
  -Publisher "CN=SignPath Foundation, ..." -EmitAppInstaller -ReleaseTag v0.1.0 -Verify
```

### 시험용 자체 서명(선택)

로컬 개발 PC 검증 전용 옵션이다. 인증서·비밀번호를 파일·명령줄·로그에 남기지 않도록 환경 변수 이름으로만 비밀번호를 받는다. 시스템 인증서 저장소는 스크립트가 건드리지 않는다.

```powershell
$env:MSIX_TEST_CERT_PW = "<비밀번호>"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package-msix.ps1 `
  -Channel direct -SkipBuild -SelfSign -CertPfx <경로.pfx> -CertPasswordEnv MSIX_TEST_CERT_PW -Verify
Remove-Item Env:MSIX_TEST_CERT_PW
```

## 3. 릴리스 CI & SignPath Foundation 준비 (`D11`)

`.github/workflows/release-msix.yml`은 태그(`v*`) 푸시 또는 수동 실행 시 `windows-latest`에서 스토어용·직접 배포용 `.msix`와 `shookshok.appinstaller`를 생성·검증한다.

- SignPath 서명 단계는 `vars.SIGNPATH_ENABLED == 'true'`일 때만 실행된다.
- 필요한 GitHub Secrets / Variables:
  - Secret: `SIGNPATH_API_TOKEN`
  - Variables: `SIGNPATH_ENABLED`, `SIGNPATH_ORGANIZATION_ID`, `SIGNPATH_PROJECT_SLUG`, `SIGNPATH_SIGNING_POLICY_SLUG`, `SIGNPATH_PUBLISHER`, `STORE_PACKAGE_NAME`, `STORE_PUBLISHER`
- SignPath Foundation 신청용 `## Code Signing Policy` 절은 루트 `README.md` 맨 끝에 들어 있다(11-distribution §10.3).

## 4. 패키지 모드 런타임 동작 차이 (`D3`)

- 순수 규칙: `crates/ink-domain/src/package.rs`
- 포트·연결: `crates/ink-app/src/ports.rs`, `crates/ink-app/src/package.rs` (`PackageRuntime`, `AutostartControl`, `Updater`, 기본 Noop)
- 저장소 위치 분기: `crates/ink-store/src/lib.rs` (`resolve_dir`가 `resolve_storage_policy` 사용, `Shookshok.portable`·`screen-ink.portable` 모두 인식)
- Windows 어댑터: `crates/ink-platform-windows/src/package.rs` (`GetCurrentPackageFullName`, WinRT `Package::Current`, `StartupTask("ShookshokStartup")`, `SHQueryUserNotificationState`, `RegisterApplicationRestart`)
- 플러그인·설정 화면: `crates/tauri-plugin-screen-ink/src/lib.rs`(`sync_autostart` 분기), `packages/ink-ui/src/lib/screens/GeneralPage.svelte`(저장 위치 `MSIX 패키지`/`설치`/`포터블`, `DisabledByUser` 한 줄 안내)

| 항목 | 비패키지 (`Unpackaged`) | 스토어 MSIX (`MsixStore`) | 직접 배포 MSIX (`MsixDirect`) |
|---|---|---|---|
| 판정 근거 | `GetCurrentPackageFullName` = `15700` | 패키지 + `Store` 서명(AppInstaller 없음) | 패키지 + `GetAppInstallerInfo()` 또는 비스토어 서명 |
| 로그인 자동 실행 | Run 레지스트리(`tauri-plugin-autostart`) | `StartupTask("ShookshokStartup")` | `StartupTask("ShookshokStartup")` |
| `Shookshok.portable` · `screen-ink.portable` | 있으면 `exe_dir\data` 사용 | **무시**(설치 폴더 읽기 전용) | **무시**(설치 폴더 읽기 전용) |
| 설정·로그 저장 위치 | 포터블(`data`) 또는 `%APPDATA%`·`%LOCALAPPDATA%` | 패키지 가상화 경로(제거 시 함께 정리) | 패키지 가상화 경로(제거 시 함께 정리) |
| 업데이트 경로 | `UpdateBackend::None`(네트워크 0) | `StoreContext` | `.appinstaller` + `PackageManager` |
