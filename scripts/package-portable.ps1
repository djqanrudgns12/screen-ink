# 같은 릴리스의 Windows 실행 파일을 포터블 ZIP으로 묶는다.
[CmdletBinding()]
param(
  [Parameter(Mandatory)][string]$ExePath,
  [Parameter(Mandatory)][string]$OutDir
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$config = Get-Content -LiteralPath (Join-Path $root 'apps/screen-ink/src-tauri/tauri.conf.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$exe = (Get-Item -LiteralPath $ExePath).FullName
$reader = [IO.BinaryReader]::new([IO.File]::OpenRead($exe))
try {
  if ($reader.ReadUInt16() -ne 0x5a4d) { throw 'Windows 실행 파일이 아닙니다.' }
  $reader.BaseStream.Position = 0x3c
  $reader.BaseStream.Position = $reader.ReadUInt32()
  if ($reader.ReadUInt32() -ne 0x4550) { throw 'PE 헤더가 올바르지 않습니다.' }
  $arch = switch ($reader.ReadUInt16()) {
    0x8664 { 'x64' }
    0xaa64 { 'arm64' }
    default { throw '지원하는 실행 파일은 x64와 arm64입니다.' }
  }
} finally {
  $reader.Dispose()
}
$license = Join-Path $root 'LICENSE'
$out = [IO.Directory]::CreateDirectory($ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutDir))
$name = "$($config.productName)_$($config.version)_${arch}-portable.zip"
$zipPath = Join-Path $out.FullName $name

# 완성된 ZIP만 최종 이름으로 옮긴다. 이미 있는 배포물은 보존한다.
$temporary = Join-Path $out.FullName ([guid]::NewGuid().ToString('N') + '.tmp')
try {
  $zip = [IO.Compression.ZipFile]::Open($temporary, [IO.Compression.ZipArchiveMode]::Create)
  try {
    [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $exe, "$($config.productName).exe") | Out-Null
    $zip.CreateEntry('Shookshok.portable') | Out-Null
    [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $license, 'LICENSE') | Out-Null
  } finally {
    $zip.Dispose()
  }
  [IO.File]::Move($temporary, $zipPath)
  Write-Host "포터블 ZIP 생성 완료 ($zipPath)"
} finally {
  if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary }
}
