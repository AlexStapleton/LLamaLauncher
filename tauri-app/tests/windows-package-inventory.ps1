param([Parameter(Mandatory=$true)][string]$Stage)
$ErrorActionPreference='Stop'
$entries=@(Get-ChildItem -LiteralPath $Stage -Filter '*.dll' -File)
$entries+=Get-Item -LiteralPath (Join-Path $Stage 'WebView2/msedgewebview2.exe')
$report=@($entries | Sort-Object Name | ForEach-Object {
    $signature=Get-AuthenticodeSignature -LiteralPath $_.FullName
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch '(^|, )O=Microsoft Corporation(,|$)') { throw "Invalid packaged Microsoft signature: $($_.Name)" }
    [ordered]@{name=$_.Name;version=$_.VersionInfo.FileVersion;sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLower();signer=$signature.SignerCertificate.Subject}
})
$json=[ordered]@{generatedAt=[DateTime]::UtcNow.ToString('o');scope='Packaged app-local VC runtime DLLs and Microsoft Fixed WebView2 executable; the package SHA256 manifest covers all WebView2 files.';binaries=$report} | ConvertTo-Json -Depth 5
[IO.File]::WriteAllText((Join-Path $Stage 'WINDOWS_PACKAGE_VERSIONS.json'),$json+"`n")
[IO.File]::WriteAllText((Join-Path $PSScriptRoot '../artifacts/WINDOWS_PACKAGE_VERSIONS.json'),$json+"`n")
