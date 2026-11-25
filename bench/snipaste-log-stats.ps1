# Snipaste 로그(splog.txt)에서 벤치마크 기준값을 뽑는다. 읽기 전용.
# 사용: powershell -ExecutionPolicy Bypass -File bench\snipaste-log-stats.ps1 -LogPath "C:\...\Snipaste-2.11.3-x64\splog.txt"
param(
    [Parameter(Mandatory = $true)][string]$LogPath,
    [string]$Since = ""   # 예: "2026-11-01" — 이 날짜 이후 줄만 집계
)

$ErrorActionPreference = 'Stop'
$lines = Get-Content -LiteralPath $LogPath -Encoding utf8
if ($Since) { $lines = $lines | Where-Object { $_.Length -gt 11 -and $_.Substring(1, 10) -ge $Since } }

function Get-Stats([int[]]$values, [string]$label) {
    if (-not $values -or $values.Count -eq 0) { return "$label : 자료 없음" }
    $s = $values | Sort-Object
    $pick = { param($q) $s[[Math]::Min($s.Count - 1, [int][Math]::Floor($s.Count * $q))] }
    "{0} : n={1} min={2} p50={3} p90={4} p95={5} max={6} (ms)" -f $label, $s.Count, $s[0], (& $pick 0.5), (& $pick 0.9), (& $pick 0.95), $s[-1]
}

function Get-Ints([string]$pattern, [int]$group = 1) {
    [int[]]($lines | Select-String -Pattern $pattern | ForEach-Object { [int]$_.Matches[0].Groups[$group].Value })
}

"기간: $($lines[0].Substring(1, 23)) ~ $($lines[-1].Substring(1, 23)), 줄 수 $($lines.Count)"
Get-Stats (Get-Ints 'Snipper: ready \[(\d+) ms\]') '캡처 오버레이 준비(단축키→표시·포커스)'
Get-Stats (Get-Ints 'Launch time: (\d+) ms') '앱 시작 시간'
Get-Stats (Get-Ints 'Preloaded \d+ files: (\d+) ms') '히스토리 미리 읽기'

"--- 캡처 결과 목적지 ---"
$lines | Select-String -Pattern 'destination: (.+)$' | ForEach-Object { $_.Matches[0].Groups[1].Value } |
    Group-Object | Sort-Object Count -Descending | ForEach-Object { "  {0,-12} {1}" -f $_.Name, $_.Count }

"--- 경고·오류 유형 ---"
$lines | ForEach-Object {
    if ($_ -match '^\[[^\]]+\] \[([WEC])\]\s+(.*)$') { "$($Matches[1]) " + ($Matches[2] -replace '\d+', 'N') }
} | Group-Object | Sort-Object Count -Descending | Select-Object -First 15 | ForEach-Object { "  {0,4}  {1}" -f $_.Count, $_.Name }

"--- 마지막 모니터 구성 ---"
$last = ($lines | Select-String -Pattern 'Screens: (\d+)' | Select-Object -Last 1)
if ($last) {
    $i = $last.LineNumber
    $lines[($i - 1)..([Math]::Min($lines.Count - 1, $i - 1 + [int]$last.Matches[0].Groups[1].Value))] | ForEach-Object { "  " + $_.Substring(26) }
}
