# Obtain app-local DLLs without installing a system redistributable. The cabinet
# coordinates are part of the hash-pinned Microsoft bundle, not a generic EXE parser.
$ErrorActionPreference='Stop'
$appRoot=Split-Path $PSScriptRoot -Parent
$inputSpec=(Get-Content (Join-Path $appRoot 'runtime-inputs.json') -Raw | ConvertFrom-Json).vcRuntime
$fixture=Join-Path $appRoot "artifacts/release-inputs/vc-$($inputSpec.version)"
New-Item -ItemType Directory -Force $fixture | Out-Null
$bundle=Join-Path $fixture 'VC_redist.x64.exe'
if (-not (Test-Path -LiteralPath $bundle)) { Invoke-WebRequest $inputSpec.url -OutFile $bundle }
if ((Get-FileHash -LiteralPath $bundle -Algorithm SHA256).Hash -ine $inputSpec.sha256) { throw 'VC runtime bundle checksum mismatch' }
function Assert-MicrosoftSignature([string]$file) {
    $signature=Get-AuthenticodeSignature -LiteralPath $file
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch '(^|, )O=Microsoft Corporation(,|$)') { throw "Invalid Microsoft signature: $(Split-Path $file -Leaf)" }
}
Assert-MicrosoftSignature $bundle
$bytes=[IO.File]::ReadAllBytes($bundle)
$offset=[int]$inputSpec.containerOffset
$length=[int]$inputSpec.containerSize
if ($offset -lt 0 -or $length -lt 36 -or [long]$offset+$length -gt $bytes.Length -or [Text.Encoding]::ASCII.GetString($bytes,$offset,4) -ne 'MSCF' -or [BitConverter]::ToUInt32($bytes,$offset+8) -ne $length) { throw 'Pinned VC cabinet layout mismatch' }
$cabinet=Join-Path $fixture 'payload.cab'
$stream=[IO.File]::Create($cabinet)
try { $stream.Write($bytes,$offset,$length) } finally { $stream.Dispose() }
$scratch=Join-Path $fixture ([guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $scratch | Out-Null
$expand=Join-Path $env:SystemRoot 'System32/expand.exe'
if ($inputSpec.payload -notmatch '^a\d+$') { throw 'Unexpected pinned cabinet member' }
& $expand $cabinet "-F:$($inputSpec.payload)" $scratch | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'VC container extraction failed' }
$payload=Join-Path $scratch $inputSpec.payload
if (-not (Test-Path -LiteralPath $payload)) { throw 'VC DLL cabinet missing' }
$dllScratch=Join-Path $scratch 'dlls'
New-Item -ItemType Directory -Path $dllScratch | Out-Null
& $expand $payload '-F:*' $dllScratch | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'VC DLL extraction failed' }
$allowed=@('concrt140.dll','msvcp140.dll','msvcp140_1.dll','msvcp140_2.dll','msvcp140_atomic_wait.dll','msvcp140_codecvt_ids.dll','vcamp140.dll','vccorlib140.dll','vcomp140.dll','vcruntime140.dll','vcruntime140_1.dll','vcruntime140_threads.dll')
$dllFiles=@(Get-ChildItem -LiteralPath $dllScratch -File)
if ($dllFiles.Count -ne $allowed.Count -or @(Get-ChildItem -LiteralPath $dllScratch -Directory).Count -ne 0) { throw 'Unexpected VC payload contents' }
$destination=Join-Path $scratch 'runtime'
New-Item -ItemType Directory -Path $destination | Out-Null
foreach ($dll in $dllFiles) {
    $name=$dll.Name -replace '_amd64$',''
    if ($dll.Name -ne "$($name)_amd64" -or $name -notin $allowed -or $dll.VersionInfo.OriginalFilename -ine $name -or $dll.VersionInfo.FileVersion -ne $inputSpec.version) { throw 'Unexpected VC DLL name/version' }
    Assert-MicrosoftSignature $dll.FullName
    $pe=[IO.File]::ReadAllBytes($dll.FullName)
    $peOffset=[BitConverter]::ToInt32($pe,60)
    if ($peOffset -lt 64 -or $peOffset+6 -gt $pe.Length -or [BitConverter]::ToUInt16($pe,$peOffset+4) -ne 0x8664) { throw 'VC runtime is not x64' }
    Copy-Item -LiteralPath $dll.FullName -Destination (Join-Path $destination $name)
}
Write-Output "VCRUNTIME_STAGE=$destination"
