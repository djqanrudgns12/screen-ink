# 화면 시안 HTML을 Edge 헤드리스로 찍어 png 폴더에 저장한다(시안을 고친 뒤 다시 실행).
# 사용: powershell -ExecutionPolicy Bypass -File docs\mockups\render.ps1 [-Only s07,s08]
param([string[]]$Only = @())

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$html = Join-Path $here 'screen-ink-mockups.html'
$out = Join-Path $here 'png'
New-Item -ItemType Directory -Force $out | Out-Null
$edge = @("${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe", "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe") |
    Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edge) { throw 'Microsoft Edge를 찾지 못했어요.' }
$profile = Join-Path $env:TEMP 'screen-ink-mockup-render'
$ids = if ($Only.Count) { ($Only -join ',') -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ } } else { 1..15 | ForEach-Object { 's{0:D2}' -f $_ } }
$base = 'file:///' + ($html -replace '\\', '/')
foreach ($id in $ids) {
    $png = Join-Path $out "$id.png"
    $args = @('--headless=new', '--disable-gpu', '--hide-scrollbars', '--no-first-run', "--user-data-dir=$profile",
        '--window-size=1440,900', '--virtual-time-budget=3000', "--screenshot=$png", "$base`?shot=1#$id")
    Start-Process -FilePath $edge -ArgumentList $args -Wait -WindowStyle Hidden
    if (Test-Path $png) { "{0}  {1:N0} bytes" -f $id, (Get-Item $png).Length } else { "$id  실패" }
}
