# 시안 자동 조작 시험(qa.html)을 Edge 헤드리스로 실행하고 PASS/FAIL 결과를 출력한다.
# 사용: powershell -ExecutionPolicy Bypass -File docs\mockups\qa.ps1
$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$edge = @("${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe", "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe") |
    Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edge) { throw 'Microsoft Edge를 찾지 못했어요.' }
$profile = Join-Path $env:TEMP 'screen-ink-mockup-qa'
$url = 'file:///' + ((Join-Path $here 'qa.html') -replace '\\', '/')
$dom = & $edge --headless=new --disable-gpu --no-first-run "--user-data-dir=$profile" '--allow-file-access-from-files' '--window-size=1500,1200' '--virtual-time-budget=20000' --dump-dom $url 2>$null | Out-String
$m = [regex]::Match($dom, '<pre id="out">([\s\S]*?)</pre>')
if (-not $m.Success) { throw '결과를 읽지 못했어요.' }
[System.Net.WebUtility]::HtmlDecode($m.Groups[1].Value)
if ($m.Groups[1].Value -notmatch '^RESULT (\d+)/(\d+) passed, 0 page errors' -or $Matches[1] -ne $Matches[2]) { exit 1 }
