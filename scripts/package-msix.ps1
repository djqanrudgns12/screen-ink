<#
.SYNOPSIS
  슉쇽 (ShookShok) Windows MSIX 포장 파이프라인(D2 · 11-distribution §5·§6·§7).

.DESCRIPTION
  1. `npm run tauri -w @screen-ink/app -- build --no-bundle`로 release exe를 빌드하거나(-SkipBuild로 기존 exe 재사용)
  2. `apps/screen-ink/src-tauri/icons/`의 기존 아이콘을 읽기 전용으로 복사하고
  3. `apps/screen-ink/src-tauri/packaging/msix/msix.config.json`에서 이름·ID·주소를 읽어 `AppxManifest.template.xml`을 치환한 뒤
  4. Windows SDK `makeappx.exe pack`(의미 검사 활성, `/nv` 미사용)으로 `.msix`를 만든다.

  안전 규칙:
  - 이 스크립트는 패키지를 설치(`Add-AppxPackage`)하거나 앱 exe를 실행하지 않는다.
  - 시스템 인증서 저장소를 변경하지 않는다.
  - 시험용 자체 서명(`-SelfSign`)은 명시적 옵션으로만 동작하며, 인증서 비밀번호는
    명령줄·파일·로그에 남기지 않고 환경 변수 이름(`-CertPasswordEnv`)으로만 읽는다.

.EXAMPLE
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package-msix.ps1 -Channel store -Verify

.EXAMPLE
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package-msix.ps1 -Channel direct -SkipBuild -OutDir D:\dev-cache\screen-ink\out -EmitAppInstaller -Verify
#>

[CmdletBinding()]
param(
  [ValidateSet('store', 'direct')]
  [string]$Channel = 'store',

  [string]$Version = '',
  [string]$DisplayName = '',
  [string]$PackageName = '',
  [string]$Publisher = '',
  [string]$PublisherDisplayName = '',
  [string]$Description = '',
  [string]$StartupTaskId = '',
  [string]$AppInstallerUri = '',

  [ValidateSet('x64', 'arm64')]
  [string]$Arch = 'x64',

  [string]$ExePath = '',
  [switch]$SkipBuild,

  [string]$OutDir = '',
  [switch]$EmitAppInstaller,
  [string]$ReleaseTag = '',
  [string]$PackageUri = '',

  [switch]$Verify,

  # 시험용 자체 서명 옵션(기본 꺼짐). 인증서·비밀번호를 파일·로그에 쓰지 않는다.
  [switch]$SelfSign,
  [string]$CertPfx = '',
  [string]$CertPasswordEnv = '',
  [string]$CertThumbprint = '',
  [string]$TimestampUrl = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$TauriDir = Join-Path $RepoRoot 'apps\screen-ink\src-tauri'
$TauriConfPath = Join-Path $TauriDir 'tauri.conf.json'
$IconsDir = Join-Path $TauriDir 'icons'
$PackagingDir = Join-Path $TauriDir 'packaging\msix'
$ConfigPath = Join-Path $PackagingDir 'msix.config.json'
$ManifestTemplatePath = Join-Path $PackagingDir 'AppxManifest.template.xml'
$AppInstallerTemplatePath = Join-Path $PackagingDir 'shookshok.appinstaller.template.xml'

function ConvertTo-MsixVersion {
  param([Parameter(Mandatory)][string]$Raw)
  $trimmed = $Raw.Trim()
  if ($trimmed.StartsWith('v') -or $trimmed.StartsWith('V')) {
    $trimmed = $trimmed.Substring(1)
  }
  $parts = $trimmed.Split('.')
  if ($parts.Length -ne 3 -and $parts.Length -ne 4) {
    throw "MSIX 버전 형식 오류('$Raw'): X.Y.Z 또는 X.Y.Z.0 이어야 합니다."
  }
  $nums = @(0, 0, 0, 0)
  for ($i = 0; $i -lt $parts.Length; $i++) {
    [uint16]$parsed = 0
    if (-not [uint16]::TryParse($parts[$i], [ref]$parsed)) {
      throw "MSIX 버전 자릿수 오류('$Raw'): 각 자리는 0~65535 정수여야 합니다."
    }
    $nums[$i] = $parsed
  }
  if ($nums[3] -ne 0) {
    throw "Microsoft Store MSIX 규칙: 네 번째 버전 자리는 0이어야 합니다('$Raw')."
  }
  return "$($nums[0]).$($nums[1]).$($nums[2]).0"
}

function Find-WindowsSdkTool {
  param(
    [Parameter(Mandatory)][string]$ToolName,
    [Parameter(Mandatory)][string]$PreferredArch
  )

  # 1. PATH에서 먼저 찾기
  $fromPath = Get-Command $ToolName -ErrorAction SilentlyContinue
  if ($fromPath -and (Test-Path $fromPath.Source)) {
    return $fromPath.Source
  }

  # 2. Windows 10/11 SDK 설치 경로(최신 버전 폴더 우선; 일부 구버전 폴더에는 makeappx 없음)
  $kitsRoots = @(
    "${env:ProgramFiles(x86)}\Windows Kits\10\bin",
    "${env:ProgramFiles}\Windows Kits\10\bin"
  ) | Where-Object { $_ -and (Test-Path $_) }

  $archOrder = @($PreferredArch, 'x64', 'arm64', 'x86') | Select-Object -Unique

  foreach ($root in $kitsRoots) {
    $versionDirs = Get-ChildItem -Path $root -Directory -ErrorAction SilentlyContinue |
      Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } |
      Sort-Object { [version]$_.Name } -Descending

    foreach ($vDir in $versionDirs) {
      foreach ($a in $archOrder) {
        $candidate = Join-Path $vDir.FullName "$a\$ToolName"
        if (Test-Path $candidate) {
          return $candidate
        }
      }
    }

    foreach ($a in $archOrder) {
      $candidate = Join-Path $root "$a\$ToolName"
      if (Test-Path $candidate) {
        return $candidate
      }
    }
  }

  return $null
}

function Resolve-ReleaseExe {
  param(
    [string]$ExplicitPath,
    [string[]]$ExecutableNames
  )
  if ($ExplicitPath) {
    if (-not (Test-Path $ExplicitPath)) {
      throw "지정한 실행 파일을 찾을 수 없습니다: $ExplicitPath"
    }
    return (Resolve-Path $ExplicitPath).Path
  }

  $dirs = @()
  if ($env:CARGO_TARGET_DIR) {
    $dirs += (Join-Path $env:CARGO_TARGET_DIR 'release')
  }
  $dirs += (Join-Path $RepoRoot 'target\release')
  $dirs += (Join-Path $TauriDir 'target\release')

  foreach ($dir in $dirs) {
    foreach ($name in $ExecutableNames) {
      $candidate = Join-Path $dir $name
      if (Test-Path $candidate) {
        return (Resolve-Path $candidate).Path
      }
    }
  }
  return $null
}

function Escape-Xml {
  param([Parameter(Mandatory)][string]$Text)
  return [System.Security.SecurityElement]::Escape($Text)
}

function Expand-Template {
  param(
    [Parameter(Mandatory)][string]$TemplatePath,
    [Parameter(Mandatory)][hashtable]$Tokens
  )
  $content = Get-Content -Path $TemplatePath -Raw -Encoding UTF8
  foreach ($key in $Tokens.Keys) {
    $content = $content.Replace("{{$key}}", (Escape-Xml -Text ([string]$Tokens[$key])))
  }
  if ($content -match '\{\{[A-Z0-9_]+\}\}') {
    throw "치환되지 않은 템플릿 자리가 남아 있습니다: $($Matches[0]) ($TemplatePath)"
  }
  return $content
}

# ── 1. 설정·메타데이터 읽기 (msix.config.json 단일 원본) ──
if (-not (Test-Path $TauriConfPath)) { throw "tauri.conf.json을 찾을 수 없습니다: $TauriConfPath" }
if (-not (Test-Path $ConfigPath)) { throw "msix.config.json을 찾을 수 없습니다: $ConfigPath" }
if (-not (Test-Path $ManifestTemplatePath)) { throw "AppxManifest.template.xml을 찾을 수 없습니다: $ManifestTemplatePath" }

$tauriConf = Get-Content -Path $TauriConfPath -Raw -Encoding UTF8 | ConvertFrom-Json
$msixConfig = Get-Content -Path $ConfigPath -Raw -Encoding UTF8 | ConvertFrom-Json
$channelCfg = $msixConfig.channels.$Channel

$resolvedVersion = ConvertTo-MsixVersion -Raw ($(if ($Version) { $Version } else { [string]$tauriConf.version }))
$resolvedDisplayName = if ($DisplayName) {
  $DisplayName
} elseif ($msixConfig.displayName) {
  [string]$msixConfig.displayName
} else {
  [string]$tauriConf.productName
}
$resolvedPackageName = if ($PackageName) {
  $PackageName
} elseif ($channelCfg.packageName) {
  [string]$channelCfg.packageName
} else {
  [string]$tauriConf.identifier
}
$resolvedPublisher = if ($Publisher) { $Publisher } else { [string]$channelCfg.publisher }
$resolvedPublisherDisplayName = if ($PublisherDisplayName) { $PublisherDisplayName } else { [string]$msixConfig.publisherDisplayName }
$resolvedDescription = if ($Description) { $Description } else { [string]$msixConfig.description }
$resolvedStartupTaskId = if ($StartupTaskId) {
  $StartupTaskId
} elseif ($msixConfig.startupTaskId) {
  [string]$msixConfig.startupTaskId
} else {
  'ShookshokStartup'
}
$artifactPrefix = if ($msixConfig.artifactPrefix) { [string]$msixConfig.artifactPrefix } else { 'shookshok' }
$appInstallerFileName = if ($msixConfig.appInstallerFileName) { [string]$msixConfig.appInstallerFileName } else { 'shookshok.appinstaller' }
$resolvedAppInstallerUri = if ($AppInstallerUri) { $AppInstallerUri } else { [string]$msixConfig.appInstallerUri }

$exeCandidates = @()
if ($msixConfig.executableCandidates) {
  foreach ($c in $msixConfig.executableCandidates) {
    if ($c) { $exeCandidates += [string]$c }
  }
}
if ($tauriConf.productName) {
  $productExe = "$($tauriConf.productName).exe"
  if ($exeCandidates -notcontains $productExe) {
    $exeCandidates = @($productExe) + $exeCandidates
  }
}
if ($exeCandidates.Count -eq 0) {
  $exeCandidates = @('shookshok.exe', 'screen-ink.exe')
}

$resolvedOutDir = if ($OutDir) { $OutDir } else { Join-Path $RepoRoot 'dist\msix' }

# ── 2. Windows SDK makeappx / signtool 찾기 ──
$makeappx = Find-WindowsSdkTool -ToolName 'makeappx.exe' -PreferredArch $Arch
if (-not $makeappx) {
  Write-Error @"
Windows SDK makeappx.exe를 찾을 수 없습니다.
설치 방법:
  1) winget install --id Microsoft.WindowsSDK.10.0.26100 -e
  2) 또는 Visual Studio Installer > 개별 구성 요소 > Windows 11 SDK(10.0.26100.0 이상)
확인 경로: C:\Program Files (x86)\Windows Kits\10\bin\<버전>\$Arch\makeappx.exe
"@
  exit 2
}

$signtool = Find-WindowsSdkTool -ToolName 'signtool.exe' -PreferredArch $Arch
if ($SelfSign -and -not $signtool) {
  Write-Error "시험용 자체 서명(-SelfSign)을 요청했지만 signtool.exe를 찾을 수 없습니다."
  exit 2
}

Write-Host "[msix] makeappx: $makeappx"
if ($signtool) {
  Write-Host "[msix] signtool: $signtool"
} else {
  Write-Host "[msix] signtool: (없음 — 무서명 포장은 정상 진행)"
}

# ── 3. Tauri release exe 빌드 또는 재사용 ──
if (-not $SkipBuild) {
  Write-Host "[msix] Tauri 릴리스 빌드 시작 (npm run tauri -w @screen-ink/app -- build --no-bundle)..."
  Push-Location $RepoRoot
  try {
    & npm run tauri -w @screen-ink/app -- build --no-bundle
    if ($LASTEXITCODE -ne 0) {
      throw "Tauri 빌드 실패 (종료 코드 $LASTEXITCODE)"
    }
  } finally {
    Pop-Location
  }
}

$resolvedExe = Resolve-ReleaseExe -ExplicitPath $ExePath -ExecutableNames $exeCandidates
if (-not $resolvedExe) {
  $joined = $exeCandidates -join ', '
  throw "릴리스 실행 파일($joined)을 찾을 수 없습니다. -SkipBuild를 빼거나 -ExePath를 지정하세요."
}
$actualExeName = (Get-Item -Path $resolvedExe).Name
Write-Host "[msix] 입력 exe: $resolvedExe (매니페스트 Executable=$actualExeName)"

# ── 4. 스테이징 폴더 조립 (아이콘은 읽기·복사만, 원본 무수정) ──
$requiredIcons = @(
  'StoreLogo.png',
  'Square44x44Logo.png',
  'Square71x71Logo.png',
  'Square150x150Logo.png'
)
foreach ($icon in $requiredIcons) {
  $srcIcon = Join-Path $IconsDir $icon
  if (-not (Test-Path $srcIcon)) {
    throw "필수 MSIX 아이콘 파일이 없습니다: $srcIcon"
  }
}

$stagingRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("shookshok-msix-stage-" + [System.Guid]::NewGuid().ToString('N'))
$assetsDir = Join-Path $stagingRoot 'Assets'
New-Item -ItemType Directory -Path $assetsDir -Force | Out-Null

try {
  Copy-Item -Path $resolvedExe -Destination (Join-Path $stagingRoot $actualExeName) -Force
  foreach ($icon in $requiredIcons) {
    Copy-Item -Path (Join-Path $IconsDir $icon) -Destination (Join-Path $assetsDir $icon) -Force
  }

  $manifestXml = Expand-Template -TemplatePath $ManifestTemplatePath -Tokens @{
    PACKAGE_NAME           = $resolvedPackageName
    PUBLISHER              = $resolvedPublisher
    VERSION                = $resolvedVersion
    ARCH                   = $Arch
    DISPLAY_NAME           = $resolvedDisplayName
    PUBLISHER_DISPLAY_NAME = $resolvedPublisherDisplayName
    EXECUTABLE             = $actualExeName
    DESCRIPTION            = $resolvedDescription
    STARTUP_TASK_ID        = $resolvedStartupTaskId
  }
  $manifestOut = Join-Path $stagingRoot 'AppxManifest.xml'
  $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
  [System.IO.File]::WriteAllText($manifestOut, $manifestXml, $utf8NoBom)

  # ── 5. makeappx pack (의미 검사 활성 — /nv 사용 안 함) ──
  New-Item -ItemType Directory -Path $resolvedOutDir -Force | Out-Null
  $channelSuffix = if ($Channel -eq 'store') { 'store' } else { 'direct' }
  $msixFileName = "${artifactPrefix}_${resolvedVersion}_${Arch}_${channelSuffix}.msix"
  $msixOutPath = Join-Path $resolvedOutDir $msixFileName

  Write-Host "[msix] makeappx pack -> $msixOutPath"
  & $makeappx pack /d $stagingRoot /p $msixOutPath /o
  if ($LASTEXITCODE -ne 0) {
    throw "makeappx pack 실패 (종료 코드 $LASTEXITCODE)"
  }

  # ── 6. (선택) 시험용 자체 서명 — 명시적 옵션일 때만, 비밀번호·인증서 내용 무출력 ──
  if ($SelfSign) {
    if ($Channel -eq 'store') {
      throw "Microsoft Store 채널(-Channel store)은 스토어가 서명하므로 -SelfSign을 쓰지 않습니다."
    }
    $signArgs = @('sign', '/fd', 'SHA256')
    if ($CertPfx) {
      if (-not (Test-Path $CertPfx)) {
        throw "지정한 PFX 파일이 없습니다."
      }
      $signArgs += @('/f', $CertPfx)
      if ($CertPasswordEnv) {
        $pw = [Environment]::GetEnvironmentVariable($CertPasswordEnv)
        if (-not $pw) {
          throw "환경 변수 '$CertPasswordEnv'가 비어 있습니다."
        }
        $signArgs += @('/p', $pw)
      }
    } elseif ($CertThumbprint) {
      $signArgs += @('/sha1', $CertThumbprint)
    } else {
      throw "-SelfSign에는 -CertPfx(선택: -CertPasswordEnv) 또는 -CertThumbprint가 필요합니다."
    }
    if ($TimestampUrl) {
      $signArgs += @('/tr', $TimestampUrl, '/td', 'SHA256')
    }
    $signArgs += $msixOutPath

    Write-Host "[msix] signtool 시험 서명 실행 중(인증서 정보는 출력하지 않음)..."
    & $signtool @signArgs | Out-Null
    if ($LASTEXITCODE -ne 0) {
      throw "signtool sign 실패 (종료 코드 $LASTEXITCODE)"
    }
    Write-Host "[msix] 시험 서명 완료."
  }

  # ── 7. (선택) .appinstaller 생성 ──
  $appInstallerOut = $null
  if ($EmitAppInstaller) {
    if (-not (Test-Path $AppInstallerTemplatePath)) {
      throw ".appinstaller 템플릿이 없습니다: $AppInstallerTemplatePath"
    }
    $tag = if ($ReleaseTag) {
      $ReleaseTag
    } else {
      $v3 = ($resolvedVersion.Split('.')[0..2]) -join '.'
      "v$v3"
    }
    $pkgUri = if ($PackageUri) {
      $PackageUri
    } else {
      "$($msixConfig.releaseBaseUrl)/$tag/$msixFileName"
    }
    $appInstallerXml = Expand-Template -TemplatePath $AppInstallerTemplatePath -Tokens @{
      APPINSTALLER_URI = $resolvedAppInstallerUri
      VERSION          = $resolvedVersion
      PACKAGE_NAME     = $resolvedPackageName
      PUBLISHER        = $resolvedPublisher
      ARCH             = $Arch
      PACKAGE_URI      = $pkgUri
    }
    $appInstallerOut = Join-Path $resolvedOutDir $appInstallerFileName
    [System.IO.File]::WriteAllText($appInstallerOut, $appInstallerXml, $utf8NoBom)
    Write-Host "[msix] .appinstaller 생성 -> $appInstallerOut"
  }

  # ── 8. (선택) makeappx unpack + 매니페스트·.appinstaller 무결성 검사 ──
  if ($Verify) {
    $unpackDir = Join-Path ([System.IO.Path]::GetTempPath()) ("shookshok-msix-verify-" + [System.Guid]::NewGuid().ToString('N'))
    try {
      Write-Host "[msix] makeappx unpack 검증 중..."
      & $makeappx unpack /p $msixOutPath /d $unpackDir /o | Out-Null
      if ($LASTEXITCODE -ne 0) {
        throw "makeappx unpack 실패 (종료 코드 $LASTEXITCODE)"
      }

      $unpackedManifestPath = Join-Path $unpackDir 'AppxManifest.xml'
      if (-not (Test-Path $unpackedManifestPath)) { throw "검증 실패: AppxManifest.xml 누락" }
      if (-not (Test-Path (Join-Path $unpackDir $actualExeName))) { throw "검증 실패: $actualExeName 누락" }
      foreach ($icon in $requiredIcons) {
        if (-not (Test-Path (Join-Path $unpackDir "Assets\$icon"))) {
          throw "검증 실패: Assets\$icon 누락"
        }
      }

      [xml]$doc = Get-Content -Path $unpackedManifestPath -Raw -Encoding UTF8
      $ns = New-Object System.Xml.XmlNamespaceManager($doc.NameTable)
      $ns.AddNamespace('m', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10')
      $ns.AddNamespace('uap', 'http://schemas.microsoft.com/appx/manifest/uap/windows10')
      $ns.AddNamespace('d', 'http://schemas.microsoft.com/appx/manifest/desktop/windows10')
      $ns.AddNamespace('r', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities')

      $ident = $doc.SelectSingleNode('/m:Package/m:Identity', $ns)
      $props = $doc.SelectSingleNode('/m:Package/m:Properties', $ns)
      $appNode = $doc.SelectSingleNode('/m:Package/m:Applications/m:Application', $ns)
      $visual = $doc.SelectSingleNode('/m:Package/m:Applications/m:Application/uap:VisualElements', $ns)
      $startup = $doc.SelectSingleNode('//d:StartupTask', $ns)
      $fullTrust = $doc.SelectSingleNode("//r:Capability[@Name='runFullTrust']", $ns)

      if ($ident.Name -ne $resolvedPackageName) { throw "매니페스트 Identity.Name 불일치" }
      if ($ident.Publisher -ne $resolvedPublisher) { throw "매니페스트 Identity.Publisher 불일치" }
      if ($ident.Version -ne $resolvedVersion) { throw "매니페스트 Identity.Version 불일치" }
      if ($props.DisplayName -ne $resolvedDisplayName) { throw "매니페스트 DisplayName 불일치" }
      if ($props.PublisherDisplayName -ne $resolvedPublisherDisplayName) { throw "매니페스트 PublisherDisplayName 불일치" }
      if (-not $appNode -or $appNode.Executable -ne $actualExeName) { throw "매니페스트 Application.Executable 불일치" }
      if (-not $visual -or $visual.DisplayName -ne $resolvedDisplayName) { throw "매니페스트 VisualElements.DisplayName 불일치" }
      if (-not $startup -or $startup.TaskId -ne $resolvedStartupTaskId) { throw "매니페스트 StartupTask($resolvedStartupTaskId) 누락" }
      if (-not $fullTrust) { throw "매니페스트 runFullTrust 기능 누락" }

      if ($appInstallerOut) {
        [xml]$aiDoc = Get-Content -Path $appInstallerOut -Raw -Encoding UTF8
        $aiNs = New-Object System.Xml.XmlNamespaceManager($aiDoc.NameTable)
        $aiNs.AddNamespace('a', 'http://schemas.microsoft.com/appx/appinstaller/2021')
        $aiRoot = $aiDoc.SelectSingleNode('/a:AppInstaller', $aiNs)
        $aiMain = $aiDoc.SelectSingleNode('/a:AppInstaller/a:MainPackage', $aiNs)
        if ($aiRoot.Uri -ne $resolvedAppInstallerUri) { throw ".appinstaller Uri 불일치" }
        if ($aiMain.Name -ne $resolvedPackageName) { throw ".appinstaller MainPackage.Name 불일치" }
        if ($aiMain.Publisher -ne $resolvedPublisher) { throw ".appinstaller MainPackage.Publisher 불일치" }
        if ($aiMain.Version -ne $resolvedVersion) { throw ".appinstaller MainPackage.Version 불일치" }
        if (-not $aiMain.Uri.EndsWith("/$msixFileName")) { throw ".appinstaller MainPackage.Uri 산출물 이름 불일치" }
      }

      Write-Host "[msix] 검증 통과: Identity=$($ident.Name) v$($ident.Version) ($($ident.ProcessorArchitecture)), DisplayName=$($props.DisplayName), PublisherDisplayName=$($props.PublisherDisplayName), Executable=$($appNode.Executable), StartupTask=$($startup.TaskId)"
    } finally {
      if (Test-Path $unpackDir) {
        Remove-Item -Path $unpackDir -Recurse -Force -ErrorAction SilentlyContinue
      }
    }
  }

  $sizeBytes = (Get-Item $msixOutPath).Length
  Write-Host "[msix] 완료: $msixOutPath ($sizeBytes bytes)"
  if ($appInstallerOut) {
    Write-Host "[msix] 완료: $appInstallerOut"
  }
} finally {
  if (Test-Path $stagingRoot) {
    Remove-Item -Path $stagingRoot -Recurse -Force -ErrorAction SilentlyContinue
  }
}
