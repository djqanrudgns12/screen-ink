# 직접 배포용 MSIX와 두 피드를 만든다. 게시와 고정 주소 호스팅은 맥에서 진행한다.
[CmdletBinding()]
param(
    [string[]]$Notes = @(),
    [string]$UploadDir = 'C:\agents\runs\W11-update\upload',
    [string]$ExePath,
    [string]$Publisher,
    [string]$CertThumbprint,
    [string]$TimestampUrl
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$tauri = Join-Path $root 'apps\screen-ink\src-tauri'
$version = (Get-Content (Join-Path $tauri 'tauri.conf.json') -Raw -Encoding UTF8 | ConvertFrom-Json).version
$config = Get-Content (Join-Path $tauri 'packaging\msix\msix.config.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$utf8 = [System.Text.UTF8Encoding]::new($false)
$upload = [IO.Path]::GetFullPath($UploadDir)
$parent = Split-Path $upload -Parent
$stage = Join-Path $parent ('release-' + [guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $stage -Force
try {
    if (!$ExePath) {
        Push-Location $root
        try {
            # npm.ps1이 -- 구분자를 소비하면 NSIS까지 빌드한다. npm.cmd에 인자 배열을 그대로 넘긴다.
            & npm.cmd @('run', 'tauri', '-w', '@screen-ink/app', '--', 'build', '--no-bundle')
            if ($LASTEXITCODE -ne 0) { throw "릴리스 빌드 실패 ($LASTEXITCODE)" }
            $target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root 'target' }
            $ExePath = Join-Path $stage 'screen-ink.exe'
            Copy-Item -LiteralPath (Join-Path $target 'release\screen-ink.exe') -Destination $ExePath
        } finally { Pop-Location }
    }
    $options = @{ Channel = 'direct'; Version = $version; ReleaseTag = "v$version"; SkipBuild = $true
        ExePath = (Resolve-Path -LiteralPath $ExePath).Path; OutDir = $stage; EmitAppInstaller = $true; Verify = $true }
    if ($Publisher) { $options.Publisher = $Publisher }
    if ($CertThumbprint) {
        if (!$Publisher) { throw '서명할 인증서의 게시자 이름을 지정해 주세요.' }
        $options.SelfSign = $true
        $options.CertThumbprint = $CertThumbprint
        $options.TimestampUrl = $TimestampUrl
    }

    & (Join-Path $PSScriptRoot 'package-msix.ps1') @options
    $msix = @(Get-ChildItem -LiteralPath $stage -Filter '*.msix' -File)
    if ($msix.Count -ne 1) { throw '포장 결과에서 MSIX 파일 한 개를 찾지 못했어요.' }
    $msix = $msix[0]
    if ($CertThumbprint -and (Get-AuthenticodeSignature -LiteralPath $msix.FullName).Status -ne 'Valid') {
        throw 'Windows에서 배포 서명을 검증하지 못했어요.'
    }

    $installer = Join-Path $stage $config.appInstallerFileName
    [xml]$xml = Get-Content -LiteralPath $installer -Encoding UTF8 -Raw
    # 설정을 꺼도 OS가 독립적으로 조회하는 시작·백그라운드 예약을 이 배포본에서는 등록하지 않는다.
    $null = $xml.AppInstaller.RemoveChild($xml.AppInstaller.UpdateSettings)
    $writer = [Xml.XmlWriter]::Create($installer, [Xml.XmlWriterSettings]@{ Encoding = $utf8; Indent = $true })
    try { $xml.Save($writer) } finally { $writer.Dispose() }

    $feed = [ordered]@{
        version = $version
        notes = @($Notes | Select-Object -First 3)
        url = $xml.AppInstaller.MainPackage.Uri
        sha256 = (Get-FileHash -LiteralPath $msix.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    $latest = Join-Path $stage 'latest-win.json'
    [IO.File]::WriteAllText($latest, ($feed | ConvertTo-Json) + [Environment]::NewLine, $utf8)
    $null = New-Item -ItemType Directory -Path $upload -Force
    foreach ($file in @($msix.FullName, $installer, $latest)) {
        Copy-Item -LiteralPath $file -Destination $upload -Force
    }
    Write-Host "릴리스 v$version 초안 파일 세 개를 모았어요. $upload"
    if (!$CertThumbprint) { Write-Host '무서명 초안입니다. 공개 전 실제 게시자로 서명한 패키지의 해시를 피드에 넣어 주세요.' }
} finally {
    # 이 실행에서 만든 임시 폴더만 정리한다.
    $actual = (Resolve-Path -LiteralPath $stage).Path
    if ((Split-Path $actual -Parent) -eq $parent -and (Split-Path $actual -Leaf) -match '^release-[a-f0-9]{32}$') {
        Remove-Item -LiteralPath $actual -Recurse -Force
    }
}
