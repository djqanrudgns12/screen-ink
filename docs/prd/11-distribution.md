# 11. 배포 채널·서명 (2026-11-26 조사)

조건(사용자): Microsoft Store에 넣는다. Windows에서 설치 파일을 나눌 때 경고 문구가 없어야 한다. 유료 서명은 싸면 받고 비싸면 뺀다.

## 1. 결론

1. **Windows 기본 채널은 Microsoft Store(MSIX 패키지)**
   - 개인 개발자 등록은 무료다(2025-09-10부터). 신분증과 셀카로 본인 확인을 한다.
   - MSIX로 내면 스토어가 Microsoft 인증서로 **대신 서명한다** → 서명 비용 0원, 설치할 때 경고가 없다.
   - 공유는 파일 대신 **스토어 링크**로 한다. 스토어에 올라간 앱은 `winget`(msstore 원본)으로도 설치된다.
2. **설치 파일을 직접 나누면서 "경고 없음 + 무료"를 함께 만족하는 정당한 방법은 없다.**
   - 서명이 없는 파일 → SmartScreen("Windows의 PC 보호")이 뜬다. 스마트 앱 컨트롤(Smart App Control)이 켜진 PC에서는 아예 막힌다.
   - 2024년부터는 비싼 EV 인증서도 SmartScreen 평판을 바로 주지 않는다. 그래서 어떤 인증서를 사도 첫날부터 경고 0은 보장되지 않고, 다운로드가 쌓이며 줄어든다.
   - 파일 공유가 꼭 필요할 때 **가장 싼 길**은 저장소를 오픈소스로 공개하고 **Certum 오픈소스 코드 서명(€49/년)** 이나 **SignPath Foundation(무료)** 으로 서명하는 것이다(§3).
3. **스토어를 더 늘린다면:** winget은 스토어에 올리면 따라온다. 국내 교사 접근성 때문에 **네이버 소프트웨어**를 검토할 만하다(§4).
   - 맥은 Mac App Store와 Homebrew 공식 목록 둘 다 막혀 있다. 그래서 자체 서명 dmg 직접 배포(2026-11-26 결정)를 유지한다.
4. **자동 업데이트는 Windows가 한다(§7).** 스토어판은 스토어가, 직접 배포판은 `.appinstaller` 파일로 Windows 앱 설치 관리자가 새 버전을 받는다. 앱 프로세스는 계속 네트워크 0이다.
5. **학교 PC 주의:** 교육청이 관리하는 PC는 Microsoft Store가 막혀 있을 수 있다. 막혀 있으면 스토어 경로가 통째로 안 된다. **[확인 필요]** 학교 PC에서 확인한다(진척도 `D6`).

## 2. 방안 비교 (Windows)

| 방안 | 비용 | 설치 때 경고 | 스마트 앱 컨트롤 | 조건·한계 | 판정 |
|---|---|---|---|---|---|
| **Microsoft Store(MSIX)** | 0원(개인 등록 무료) | 없음(스토어 설치) | 통과(Microsoft 서명) | Tauri가 MSIX를 만들지 않아 직접 포장해야 함(§5). 공유는 링크. 스토어가 막힌 PC에서는 못 씀 | **채택(기본)** |
| Microsoft Store(EXE/MSI 링크) | 인증서값 | 없음 | 통과 | 신뢰 루트 CA 인증서로 서명한 설치 파일 + HTTPS 주소 필요(자체 서명 불가) | 기각(유료 인증서 필요) |
| Certum 오픈소스 코드 서명 | €49/년(클라우드) | 처음엔 SmartScreen 경고, 평판이 쌓이면 사라짐 | 통과(서명됨) | 비상업 오픈소스·개인. 표시 이름 "Open Source Developer, <이름>". 유효 1년 | **보조(파일 공유가 필요할 때)** |
| SignPath Foundation | 무료 | 위와 같음 | 통과 | OSI 라이선스·공개 저장소·CI 빌드·서명마다 승인. 게시자 = SignPath Foundation | 보조 후보 |
| Azure Artifact Signing(Trusted Signing) | $9.99/월 | 위와 같음 | 통과 | **개인은 미국·캐나다만** → 한국 개인 불가 | 기각(자격 없음) |
| 일반 OV / EV 인증서 | €209~ / €379~ /년 | 위와 같음(EV도 즉시 평판 없음) | 통과 | — | 기각(비쌈) |
| 서명 없음(지금) | 0원 | 경고 | **차단** | — | 학교 내부 시험용으로만 |

> SmartScreen 경고를 기술적으로 우회하는 방법(파일 표시 지우기 등)은 쓰지 않는다. 받는 사람 PC의 보안 기능을 피해 가는 것이라 신뢰를 잃는다.

## 3. 파일 직접 공유를 한다면

1. 저장소 공개 결정(06 §2.2)이 먼저다. Certum 오픈소스와 SignPath 모두 공개 오픈소스가 조건이다.
2. Certum(€49/년)을 쓰면 서명은 클라우드(SimplySign)에서 한다. 실물 카드는 필요 없다.
   - NSIS 설치본·포터블 exe에 서명한다(Tauri `bundle.windows.signCommand`).
3. 서명 뒤에도 SmartScreen 경고가 처음에는 남을 수 있다. 같은 인증서로 계속 배포해 평판을 쌓는다. 인증서를 바꾸면 평판이 처음부터 다시 쌓인다.
4. 이 경로는 진척도 `D7`(사용자 결정)이다.

## 4. 추가 스토어·채널

| 채널 | 판정 | 이유 |
|---|---|---|
| **winget** | 자동(스토어 경유) | 스토어 앱은 `winget install <스토어 ID> --source msstore`로 설치된다. 직접 설치 파일로 `winget-pkgs`에 올리는 것은 서명한 뒤에 검토한다(무서명은 검증에서 걸릴 수 있음) |
| **네이버 소프트웨어**(software.naver.com) | 검토 | 국내 교사가 찾는 곳. 다만 내려받은 파일 자체는 SmartScreen 대상이다 → 스토어 링크를 안내하거나 서명한 뒤에. 등록 절차·조건은 **[확인 필요]** |
| Chocolatey·Scoop | 보류 | 대상(교사)과 맞지 않음 |
| Mac App Store | **불가** | 남의 앱을 읽는 손쉬운 사용(AX) API는 앱 샌드박스와 맞지 않는다(Apple DTS 답변). 게다가 Apple Developer Program 연 $99 |
| Homebrew 공식 cask | **불가** | 2026-09-01부터 공증되지 않은 앱은 막는다. 자체 tap은 가능하지만 Gatekeeper 경고는 그대로 |
| Setapp 등 맥 유료 마켓 | 보류 | 공증(유료 Apple 계정) 필요 |

## 5. MSIX로 내기 위해 바꿀 것 (진척도 `D2`·`D3`)

| 항목 | 지금 | MSIX에서 |
|---|---|---|
| 포장 | Tauri NSIS | release exe + 자원 → `AppxManifest.xml`(데스크톱 앱, `runFullTrust`) → Windows SDK `makeappx pack` → **서명 없이** Partner Center에 올린다(스토어가 서명). 로컬 설치 시험은 개발 PC에서만 자체 서명 + 신뢰 등록 |
| 로그인 자동 실행 | `tauri-plugin-autostart`(Run 레지스트리) | 패키지 안의 Run 키 쓰기는 가상화되어 **효과가 없다** → 매니페스트 `windows.startupTask` + `StartupTask` API. 패키지로 실행 중인지에 따라 나눈다 |
| 포터블 표시 파일 | 실행 폴더의 `screen-ink.portable` | 설치 폴더가 읽기 전용 → 패키지일 때는 무시한다 |
| 설정·기록 위치 | `%APPDATA%\screen-ink` | 패키지 전용으로 가상화된다(앱 동작은 같고, 제거하면 함께 지워짐). 05 CRUD의 "제거 시" 문구를 맞춘다 |
| 업데이트 | (2차) Updater 포트 | 스토어가 담당한다 → 스토어 빌드에서는 끈다 |
| 전역 단축키·화면 캡처·UIA·캡처 제외 | Win32 | full trust 데스크톱 앱이면 같다. **[확인 필요]** 인증 통과로 확인 |
| 스토어 등록 자료(`D4`) | — | 개인정보 처리방침 URL(앱은 네트워크를 쓰지 않음 → "수집 없음" 한 쪽, GitHub Pages 등), 실제 화면 스크린숏, 연령 등급 설문, 한국어·영어 설명 |

## 6. 서명한 MSIX 한 파일로 나누기 (2026-11-26 확인)

**결론: 된다.** 공인 인증 기관(CA) 인증서로 서명하면 `.msix` 한 파일만 나누면 된다. x64와 arm64를 함께 낼 때도 `.msixbundle` 한 파일이다. 받는 사람은 **내려받기 → 더블클릭 → [설치]** 만 한다.
- 예전에 번거로웠던 경우(`.bat`·PowerShell로 인증서를 먼저 넣고 다시 설치 실행)는 **자체 서명 인증서**일 때다. 그때는 받는 사람 PC의 "신뢰할 수 있는 사람" 인증서 저장소에 인증서를 먼저 넣어야 한다.
- 서명이 아예 없는 MSIX는 설치 자체가 안 된다.

| 조건 | 내용 | 확인 |
|---|---|---|
| 인증서 | 공인 CA 인증서(예: Certum 오픈소스 €49/년)로 서명 → "Windows가 이미 신뢰, 인증서 배포 필요 없음"(Microsoft 문서). 매니페스트의 `Publisher`가 인증서 주체와 **글자 그대로 같아야** 한다 | 문서 |
| Windows 버전 | Windows 10 2004 이상과 Windows 11은 스토어 밖 서명 패키지 설치(사이드로드)가 기본으로 켜져 있다 | 문서 |
| 설치 도구 | 앱 설치 관리자(App Installer)는 Windows 10/11에 기본으로 있다. 내 PC의 파일 설치는 기본 허용이고, 관리자 정책으로 막을 수 있다(`MsixAllowedZones` 등) | 문서 · 학교 PC **[확인 필요]** |
| 관리자 권한 | 필요 없음(사용자별 설치), 제거 깔끔 | MSIX 특성 |
| VC++ 런타임 | 필요 없음. Tauri 기본값 `staticVCRuntime = true`(tauri-utils 2.10.4)이고, 우리 exe는 `VCRUNTIME140.dll`을 쓰지 않는다(2026-11-26 확인). UCRT는 Windows 10 이상에 들어 있다 | 실측 |
| WebView2 | 필요. Windows 11은 기본 내장, Windows 10은 대개 Edge와 함께 있다. MSIX는 WebView2 설치기를 대신 돌려 주지 못한다 → 없으면 앱이 알아채고 안내해야 한다 | 오래된 Windows 10 **[확인 필요]** |
| 웹 한 번 클릭 설치 | `ms-appinstaller:` 링크는 2023-12부터 기본으로 꺼짐 → 파일을 받아 더블클릭하는 방식만 권한다 | 문서 |
| 경고 | 앱 설치 관리자는 게시자 이름을 보여 준다. 내려받은 `.msix`에 SmartScreen 평판 검사가 붙는지는 Microsoft 문서에 없다 → 서명한 뒤 실제로 받아서 확인한다 | **[확인 필요]** |
| 업데이트 | `.msix`만 나누면 자동 업데이트가 없다. 대신 **`.appinstaller` 파일 하나**를 나누면 설치도 되고 자동 업데이트도 된다(§7) | 문서 |

주의:
- 스토어판과 직접 배포판은 매니페스트 `Publisher`가 다르다(스토어는 Partner Center가 준 게시자 이름, 직접 배포판은 인증서 주체). 그래서 Windows는 **서로 다른 앱**으로 본다. 한 PC에 둘 다 깔지 않도록 안내한다.
- 공인 인증서 없이(무료로) 한 파일 무경고 배포를 하는 방법은 여전히 없다. 무료로 가려면 스토어 링크다(§1).
- 서명한 NSIS 설치 파일(`setup.exe`)도 한 파일이고 인증서 비용은 같다. MSIX는 관리자 권한이 필요 없고 제거가 깔끔하며, 스토어용 포장과 같은 파이프라인(`D2`)을 쓴다.

## 7. 자동 업데이트 — 운영체제가 하게 한다 (2026-11-26 결정)

**원칙: 업데이트는 앱이 아니라 Windows(스토어·앱 설치 관리자)가 한다.**
- 이러면 앱 프로세스는 계속 네트워크 0이다. R-NET-1·R-NET-2를 그대로 지킨다.
- 학교망 SSL 검사 때문에 업데이트가 실패해도 앱 동작에는 영향이 없다.

| 받는 길 | 업데이트 방식 | 앱에 넣을 코드 | 비용 | 조건 |
|---|---|---|---|---|
| **① Microsoft Store**(기본) | 스토어가 자동 갱신 | 없음 | 0원 | 스토어가 열리는 PC |
| **② 서명 MSIX + `.appinstaller`**(스토어가 막힌 PC·파일 공유) | `.appinstaller` 파일 하나를 나눈다. 받는 사람이 더블클릭하면 앱이 설치되고, 그 뒤에는 Windows가 그 주소를 보고 새 버전을 받는다(앱 시작 때 + 백그라운드 8시간마다) | 없음(선택: §7.2) | Certum €49/년 | 공인 인증서 → 저장소 공개(§8). `.appinstaller`와 `.msix`를 고정 HTTPS 주소에 호스팅 |
| ③ NSIS·포터블 + 앱 안 업데이터 | 앱이 GitHub 릴리스를 확인하고 설치(06 §2, Tidy 이식) | 많음(네트워크) | 0원(minisign 키) | **2차**(R-NET-3). 첫 설치 때 SmartScreen 경고 |

### 7.1 ② `.appinstaller` 구성

```xml
<AppInstaller xmlns="http://schemas.microsoft.com/appx/appinstaller/2021"
  Uri="https://<호스트>/screen-ink.appinstaller" Version="0.2.0.0">
  <MainPackage Name="<패키지 이름>" Publisher="<Certum 인증서 주체와 같게>"
    Version="0.2.0.0" ProcessorArchitecture="x64"
    Uri="https://<호스트>/screen-ink_0.2.0.0_x64.msix" />
  <UpdateSettings>
    <OnLaunch HoursBetweenUpdateChecks="0" ShowPrompt="false" UpdateBlocksActivation="false" />
    <AutomaticBackgroundTask />
  </UpdateSettings>
</AppInstaller>
```

- **호스팅:** 저장소를 공개하면 GitHub Pages나 Releases를 쓴다. `.appinstaller`의 주소는 **바뀌면 안 된다.** 설치된 앱이 그 주소를 계속 본다.
- **릴리스할 때마다:** 버전을 올린 `.msix`를 올리고 `.appinstaller`의 `Version`과 `Uri`를 바꾼다. 이 작업은 릴리스 스크립트가 한다(`D9`).
- **스키마:** 2021 스키마를 쓴다. 2017 스키마에는 `ShowPrompt`·`UpdateBlocksActivation`이 없다.
- **나누는 방법:** 받는 사람에게는 `.msix` 대신 **`.appinstaller` 파일 하나**를 준다. `ms-appinstaller:` 링크는 꺼져 있으니 파일을 받아 더블클릭하게 안내한다.
- **[확인 필요]**
  - 트레이에 상주한 채 실행 중일 때 업데이트가 언제 적용되는지. 백그라운드로 받아 두었다가 다음 실행 때 적용되는 것으로 보이지만 문서로 확인하지 못했다.
  - GitHub `releases/latest/download/…` 리디렉션 주소를 App Installer가 따라가는지
  - 학교 PC 정책

### 7.2 (선택) 앱이 "새 버전 있어요"를 알려 주기

- 상주 앱은 다시 시작할 일이 드물다. 그래서 백그라운드로 받아 둔 업데이트가 오래 미뤄질 수 있다.
- 이때 앱이 Windows API(`Package.CheckUpdateAvailabilityAsync`)로 물어보고, 섬에 "다시 시작하면 새 버전이에요 [다시 시작]"을 띄울 수 있다.
- 통신은 Windows가 하고 우리 실행 파일에는 HTTP가 들어가지 않는다. 다만 "앱이 업데이트를 확인한다"는 점에서 R-NET-1과 부딪힌다. 그래서 **사용자 결정** 항목으로 둔다(진척도 `D9` 메모).
- 모드(캡처·판서·녹화) 중에는 묻지도, 띄우지도 않는다(06 §2.1과 같은 원칙).

## 8. Certum 오픈소스 인증서 — 비용·범위 (2026-11-26 조사)

| 항목 | 내용 |
|---|---|
| 가격 | **클라우드(SimplySign) €49 ≈ 약 7.2만 원**(1 € ≈ 1,475원, XE 2026-11-25). 해외 카드 결제 수수료를 더하면 약 7.3만 원. 실물 카드 세트는 €69 ≈ 약 10.2만 원 |
| 기간 | **1년**. 매년 갱신(갱신 때도 같은 값 예상 **[확인 필요]**) |
| 누구에게 | **개인**에게만. 인증서에 "Open Source Developer, <이름>"이 찍힌다 |
| 다른 앱에도? | 인증서는 앱이 아니라 **사람**에게 나온다 → 유효 기간 안에는 다른 앱도 같은 인증서로 서명할 수 있다. **조건은 "오픈소스·비상업"**: 상업적으로 배포하는 소프트웨어에 서명하면 인증서가 **취소**된다(Certum 문서). 그러니 공개 오픈소스이고 돈을 받지 않는 앱이라면 추가 비용 없이 같이 쓴다. 비공개·유료 앱은 안 된다(일반 인증서 €209~ ≈ 31만 원~). 한 인증서로 여러 프로젝트를 서명하는 것을 명시한 조항은 찾지 못함 → 신청할 때 Certum에 확인 **[확인 필요]** |
| 신청 서류 | 유효한 신분증 + 공과금 고지서(주소 증명) + 공개 오픈소스 프로젝트 주소(신청자와의 관계가 드러나야 함). 한국 거주자 서류 요건 **[확인 필요]** |
| 스토어와의 관계 | 스토어판(MSIX)은 스토어가 서명하므로 이 인증서가 필요 없다. 이 인증서는 ② 직접 배포판에만 쓴다. 스토어에서 **유료로 팔면** 상업 배포가 되므로, 그 경우 직접 배포판 서명에 오픈소스 인증서를 쓰는 것이 약관에 걸릴 수 있다 **[확인 필요]** |
| 저장소 공개와의 관계 | Certum은 공개 오픈소스가 조건이다 → 06 §2.2의 "코드 비공개 + 릴리스 전용 공개 저장소(B안)"와는 함께 쓸 수 없다. ②를 택하면 A안(공개)이 된다 |

## 9. 출처

- 개인 개발자 무료 등록: [Windows Developer Blog 2025-09-10](https://blogs.windows.com/windowsdeveloper/2025/09/10/free-developer-registration-for-individual-developers-on-microsoft-store/), [storedeveloper.microsoft.com](https://storedeveloper.microsoft.com/)
- MSIX는 스토어가 서명, EXE/MSI는 신뢰 루트 CA 서명 필요: [MSIX 패키지 요구사항](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/app-package-requirements), [Code signing options](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options), [Win32 앱 스토어 배포](https://learn.microsoft.com/en-us/windows/apps/distribute-through-store/how-to-distribute-your-win32-app-through-microsoft-store)
- Tauri는 EXE/MSI만 생성, 스토어는 서명된 오프라인 설치 파일 필요: [Tauri — Microsoft Store](https://v2.tauri.app/distribute/microsoft-store/)
- MSIX 자동 실행은 startupTask: [StartupTasks 요소](https://learn.microsoft.com/en-us/uwp/schemas/appxpackage/uapmanifestschema/element-desktop-startuptasks), [Advanced Installer — MSIX 시작 프로그램](https://www.advancedinstaller.com/startup-programs-msix.html)
- 스마트 앱 컨트롤(평판 → 서명 확인, 개별 예외 없음): [Microsoft 지원 FAQ](https://support.microsoft.com/en-us/windows/security/threat-malware-protection/smart-app-control-frequently-asked-questions)
- EV도 즉시 평판 없음(2024): [ToDesktop PSA](https://www.todesktop.com/blog/posts/windows-apps-psa-ev-certs-do-not-grant-immediate-reputation-anymore), [Microsoft Q&A](https://learn.microsoft.com/en-us/answers/questions/417016/reputation-with-ov-certificates-and-are-ev-certifi)
- Azure Artifact Signing 개인 자격(미국·캐나다): [Code signing options](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options), [Microsoft Q&A](https://learn.microsoft.com/en-us/answers/questions/6020935/artifact-signing)
- Certum 오픈소스 서명 가격: [Certum Shop — Open Source Code Signing in the Cloud](https://shop.certum.eu/open-source-code-signing-on-simplysign.html)
- SignPath Foundation 조건: [signpath.org/terms](https://signpath.org/terms.html)
- winget 무서명 설치 파일(공식 규정 아님, 사례): [astral-sh/uv #11456](https://github.com/astral-sh/uv/issues/11456)
- Homebrew 공증 안 된 cask 차단(2026-09-01): [Homebrew 토론 #6482](https://github.com/orgs/Homebrew/discussions/6482), [Hacker News](https://news.ycombinator.com/item?id=45907259)
- Mac App Store와 손쉬운 사용 API: [Apple Developer Forums](https://developer.apple.com/forums/thread/756130)
- 네이버 소프트웨어: [네이버 보도자료](https://navercorp.com/media/pressReleasesDetail?seq=28904)
- 서명 패키지 더블클릭 설치·사이드로드 기본 켬(2004 이상): [MSIX on Windows 10 and 11](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/msix-windows10-windows11), [Sideload apps](https://learn.microsoft.com/en-us/windows/application-management/sideload-apps-in-windows)
- CA 인증서면 인증서 배포 불필요·자체 서명은 신뢰할 수 있는 사람 저장소에 배포: [웹에서 Windows 앱 설치](https://learn.microsoft.com/en-us/windows/msix/app-installer/installing-windows10-apps-web)
- 앱 설치 관리자 보안(SmartScreen URL 검사·영역 정책): [App Installer 보안 기능](https://learn.microsoft.com/en-us/windows/msix/app-installer/app-installer-security-features)
- `ms-appinstaller:` 기본 꺼짐(2023-12): [MSRC 블로그](https://www.microsoft.com/en-us/msrc/blog/2023/12/microsoft-addresses-app-installer-abuse)
- Tauri 정적 VC 런타임: 로컬 `tauri-build 2.7.6`·`tauri-utils 2.10.4` 소스(`build.windows.staticVCRuntime` 기본 `true`)
- `.appinstaller` 업데이트 설정(OnLaunch·AutomaticBackgroundTask 8시간): [AutomaticBackgroundTask](https://learn.microsoft.com/en-us/uwp/schemas/appinstallerschema/element-s4-automaticbackgroundtask), [update-settings](https://github.com/microsoftdocs/msix-docs/blob/main/msix-src/app-installer/update-settings.md), [App Installer 파일 개요](https://learn.microsoft.com/en-us/windows/msix/app-installer/app-installer-file-overview)
- GitHub 릴리스로 `.appinstaller` 호스팅(독립 .msix만으로는 자동 업데이트 없음): [msix-docs #528](https://github.com/MicrosoftDocs/msix-docs/issues/528), 2021 스키마: [WindowsAppSDK #5125](https://github.com/microsoft/WindowsAppSDK/discussions/5125)
- 앱에서 업데이트 확인: [App Consult 블로그](https://techcommunity.microsoft.com/t5/modern-work-app-consult-blog/implements-silent-auto-update-feature-as-far-as-possible-for/ba-p/1334405)
- Certum 서류·상업 배포 서명 시 취소: [Certum 서류 안내](https://support.certum.eu/en/code-signing-required-documents/)
- 환율: [XE EUR→KRW](https://www.xe.com/en-us/currencyconverter/convert/?Amount=1&From=EUR&To=KRW)

가격·자격은 2026-11-26 기준이다. 사기 전에 각 판매처 공식 페이지에서 다시 확인한다.
