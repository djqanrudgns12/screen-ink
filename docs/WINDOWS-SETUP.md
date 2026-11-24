# Windows PC 개발 환경 준비

학교 Windows PC에서 한 번만 하면 된다. 기준 문서: [Tauri 준비물](https://v2.tauri.app/start/prerequisites/).

## 1. 설치 (PowerShell)

```powershell
winget install --id Git.Git -e
winget install --id OpenJS.NodeJS.LTS -e
winget install --id Rustlang.Rustup -e
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

- 마지막 줄은 **Visual Studio Build Tools의 "C++를 사용한 데스크톱 개발"**이다. Rust가 Windows용 실행 파일을 만들 때 필요하다.
- WebView2는 Windows 11에 기본으로 들어 있다(Windows 10이면 Microsoft 사이트에서 "Evergreen" 설치).
- Tidy-Task-Local을 빌드한 PC라면 대부분 이미 설치돼 있다 → 2번 확인만 하면 된다.
- **관리자 권한이 없으면**: Git·Node·Rust는 사용자 계정에도 설치되지만 Build Tools는 관리자 권한이 필요하다. 학교 정책상 막히면 정보 담당 선생님께 요청하거나 집 PC를 쓴다.

## 2. 확인 (새 PowerShell 창에서)

```powershell
git --version
node -v
rustc -V
cargo -V
rustup default stable-msvc
```

`node -v`는 22 이상이면 된다. `rustc`가 없다고 나오면 PowerShell을 닫았다가 다시 연다.

## 3. 저장소 받기

```powershell
cd $HOME\Desktop
git clone https://github.com/djqanrudgns12/screen-ink.git
cd screen-ink
```

비공개 저장소라 처음 한 번 GitHub 로그인 창이 뜬다(브라우저로 로그인).

## 4. 실행 (코드가 올라간 뒤)

```powershell
npm install
npm run tauri dev
```

## 5. 시험할 조건

- 배율이 다른 듀얼 모니터(예: 100% + 150%) — 디스플레이 설정에서 바꿔 가며
- PowerPoint 슬라이드 쇼 위에서 F1·Alt+1·Alt+2
- 관리자가 아닌 표준 사용자 계정
- 결과·오류 메시지는 그대로 복사해 대화에 붙여 주면 된다

## 선택: 원격 빌드 확인

Windows PC에 OpenSSH 서버를 켜면 맥에서 빌드·로그를 원격으로 볼 수 있다(관리자 권한 필요, 학교 PC는 정책상 막힐 수 있음).
화면을 직접 조작하는 시험은 어차피 손으로 해야 하므로 필수는 아니다.
