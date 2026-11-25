# 프로세스 트리(자식 포함, 예: Tauri 앱의 msedgewebview2.exe)의 메모리·CPU를 일정 간격으로 잰다. 읽기 전용.
# 사용: powershell -ExecutionPolicy Bypass -File bench\sample-process.ps1 -Name Snipaste -Seconds 60 -OutCsv bench\results\snipaste-idle.csv
param(
    [Parameter(Mandatory = $true)][string]$Name,   # 확장자 없는 프로세스 이름(예: Snipaste, ICanZoomit, screen-ink)
    [int]$Seconds = 60,
    [int]$IntervalMs = 1000,
    [string]$OutCsv = ""
)

$ErrorActionPreference = 'Stop'
$roots = @(Get-Process -Name $Name -ErrorAction SilentlyContinue)
if ($roots.Count -eq 0) { throw "실행 중인 '$Name' 프로세스가 없습니다." }

function Get-TreeIds([int[]]$rootIds) {
    $all = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId
    $ids = New-Object System.Collections.Generic.HashSet[int]
    $queue = New-Object System.Collections.Generic.Queue[int]
    foreach ($r in $rootIds) { [void]$ids.Add($r); $queue.Enqueue($r) }
    while ($queue.Count -gt 0) {
        $p = $queue.Dequeue()
        foreach ($c in ($all | Where-Object { $_.ParentProcessId -eq $p })) {
            if ($ids.Add([int]$c.ProcessId)) { $queue.Enqueue([int]$c.ProcessId) }
        }
    }
    , $ids
}

$cores = [Environment]::ProcessorCount
$rows = New-Object System.Collections.Generic.List[object]
$prevCpu = @{}
$prevTime = [DateTime]::UtcNow
$deadline = $prevTime.AddSeconds($Seconds)

while ([DateTime]::UtcNow -lt $deadline) {
    $ids = Get-TreeIds ($roots | ForEach-Object { $_.Id })
    $procs = @(Get-Process -Id ([int[]]$ids) -ErrorAction SilentlyContinue)
    $now = [DateTime]::UtcNow
    $elapsed = ($now - $prevTime).TotalSeconds
    $cpuDelta = 0.0
    foreach ($p in $procs) {
        $t = $p.TotalProcessorTime.TotalSeconds
        if ($prevCpu.ContainsKey($p.Id)) { $cpuDelta += ($t - $prevCpu[$p.Id]) }
        $prevCpu[$p.Id] = $t
    }
    $rows.Add([pscustomobject]@{
        time          = $now.ToString('o')
        processes     = $procs.Count
        private_mb    = [Math]::Round((($procs | Measure-Object PrivateMemorySize64 -Sum).Sum) / 1MB, 1)
        workingset_mb = [Math]::Round((($procs | Measure-Object WorkingSet64 -Sum).Sum) / 1MB, 1)
        handles       = ($procs | Measure-Object HandleCount -Sum).Sum
        cpu_percent   = if ($rows.Count -eq 0) { $null } else { [Math]::Round(100 * $cpuDelta / ($elapsed * $cores), 2) }
    })
    $prevTime = $now
    Start-Sleep -Milliseconds $IntervalMs
}

if ($OutCsv) {
    New-Item -ItemType Directory -Force (Split-Path -Parent $OutCsv) | Out-Null
    $rows | Export-Csv -LiteralPath $OutCsv -NoTypeInformation -Encoding UTF8
}

$cpu = @($rows | Where-Object { $null -ne $_.cpu_percent } | ForEach-Object { $_.cpu_percent })
"대상: $Name (프로세스 {0}개, 표본 {1}개, {2}초)" -f ($rows | Measure-Object processes -Maximum).Maximum, $rows.Count, $Seconds
"전용 메모리 평균 {0} MB · 최대 {1} MB" -f [Math]::Round(($rows | Measure-Object private_mb -Average).Average, 1), ($rows | Measure-Object private_mb -Maximum).Maximum
"작업 집합 평균 {0} MB · 최대 {1} MB" -f [Math]::Round(($rows | Measure-Object workingset_mb -Average).Average, 1), ($rows | Measure-Object workingset_mb -Maximum).Maximum
"핸들 처음 {0} → 끝 {1}" -f $rows[0].handles, $rows[-1].handles
if ($cpu.Count -gt 0) { "CPU 평균 {0}% · 최대 {1}% (전체 코어 {2}개 기준)" -f [Math]::Round(($cpu | Measure-Object -Average).Average, 2), ($cpu | Measure-Object -Maximum).Maximum, $cores }
