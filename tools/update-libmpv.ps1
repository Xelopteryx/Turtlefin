# Met à jour la libmpv de Windows fournie par le dépôt (vendor\libmpv, voir son README.md).
#
# Usage (depuis la racine du dépôt) :
#   powershell -ExecutionPolicy Bypass -File tools\update-libmpv.ps1 C:\chemin\libmpv-2.dll
#   powershell -ExecutionPolicy Bypass -File tools\update-libmpv.ps1 C:\chemin\libmpv-2.dll -Arch x86
#   powershell -ExecutionPolicy Bypass -File tools\update-libmpv.ps1 -Download
# -Download : dernière version de shinchiro/mpv-winbuild-cmake (64 bits seulement ; empreinte de
# l'archive vérifiée avec celle publiée par GitHub). Il faut 7-Zip.
# Ensuite : `cargo build` pose la nouvelle DLL à côté de l'exécutable.
param(
    [string]$Dll,
    [ValidateSet("x64", "x86")][string]$Arch = "x64",
    [switch]$Download,
    # Provenance écrite dans SOURCE.txt (par défaut : le chemin ou l'archive téléchargée).
    [string]$Source,
    # 32 bits : accepter une version téléchargée quand même (voir plus bas).
    [switch]$Force
)
$ErrorActionPreference = "Stop"
$root = (Resolve-Path "$PSScriptRoot\..").Path
$name = if ($Arch -eq "x64") { "x86_64" } else { "i686" }
$out = Join-Path $root "vendor\libmpv\windows-$name"
$tmp = Join-Path ([IO.Path]::GetTempPath()) "turtlefin-libmpv-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Force $tmp | Out-Null

try {
    if ($Download) {
        # Les versions 32 bits publiées depuis juillet 2026 plantent au démarrage (« OpenSSL internal
        # error: assertion failed: lock != NULL ») : celle du dépôt (10 juin 2026) est la dernière bonne.
        if ($Arch -eq "x86" -and -not $Force) {
            throw "32 bits : les versions publiées après le 10 juin 2026 plantent au démarrage. Donne une DLL essayée (chemin), ou -Force."
        }
        $headers = @{ "User-Agent" = "turtlefin-build" }
        $rel = $null
        foreach ($i in 1..4) {
            try { $rel = Invoke-RestMethod "https://api.github.com/repos/shinchiro/mpv-winbuild-cmake/releases/latest" -Headers $headers; break }
            catch { Start-Sleep -Seconds 5 }
        }
        if (-not $rel) { throw "GitHub ne répond pas (réessaie plus tard)" }
        $asset = $rel.assets | Where-Object { $_.name -like "mpv-dev-$name-2*.7z" } | Select-Object -First 1
        if (-not $asset) { throw "aucune archive mpv-dev-$name dans la version $($rel.tag_name)" }
        if (-not ($asset.digest -like "sha256:*")) { throw "GitHub ne publie pas d'empreinte pour $($asset.name) : refusé" }
        $archive = Join-Path $tmp $asset.name
        Write-Host "Téléchargement de $($asset.name)…"
        Invoke-WebRequest $asset.browser_download_url -OutFile $archive -UseBasicParsing
        $got = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLower()
        if ($got -ne $asset.digest.Substring(7).ToLower()) { throw "empreinte de $($asset.name) différente de celle publiée : refusé" }
        $7z = (Get-Command 7z -ErrorAction SilentlyContinue).Source
        if (-not $7z) { $7z = "C:\Program Files\7-Zip\7z.exe" }
        if (-not (Test-Path $7z)) { throw "7-Zip introuvable" }
        & $7z e $archive "-o$tmp" libmpv-2.dll -y | Out-Null
        $Dll = Join-Path $tmp "libmpv-2.dll"
        if (-not (Test-Path $Dll)) { throw "libmpv-2.dll absente de $($asset.name)" }
        if (-not $Source) { $Source = "shinchiro/mpv-winbuild-cmake $($rel.tag_name) ($($asset.name))" }
    }
    if (-not $Dll) { throw "donne le chemin d'une libmpv-2.dll, ou -Download" }
    $Dll = (Resolve-Path $Dll).Path
    if (-not $Source) { $Source = "copiée depuis $Dll" }

    # La DLL est-elle bien pour cette architecture ? (en-tête PE : 0x8664 = 64 bits, 0x14c = 32 bits)
    $bytes = [IO.File]::ReadAllBytes($Dll)
    if ($bytes.Length -lt 64 -or $bytes[0] -ne 0x4D -or $bytes[1] -ne 0x5A) { throw "$Dll n'est pas une DLL Windows" }
    $pe = [BitConverter]::ToInt32($bytes, 0x3C)
    $machine = [BitConverter]::ToUInt16($bytes, $pe + 4)
    $want = if ($Arch -eq "x64") { 0x8664 } else { 0x14C }
    if ($machine -ne $want) {
        $seen = if ($machine -eq 0x8664) { "64 bits" } elseif ($machine -eq 0x14C) { "32 bits" } else { "autre" }
        throw "$Dll est une DLL $seen, pas $Arch (option -Arch)"
    }

    # Compression (GitHub refuse les fichiers de plus de 100 Mo).
    $gz = Join-Path $tmp "libmpv-2.dll.gz"
    $fs = [IO.File]::Create($gz)
    $z = New-Object IO.Compression.GZipStream($fs, [IO.Compression.CompressionLevel]::Optimal)
    $z.Write($bytes, 0, $bytes.Length)
    $z.Dispose()
    $fs.Dispose()
    $size = (Get-Item $gz).Length
    if ($size -ge 100MB) { throw "même compressée, la DLL fait $([math]::Round($size / 1MB)) Mo : GitHub la refuserait" }
    if ($size -ge 50MB) { Write-Warning "compressée : $([math]::Round($size / 1MB)) Mo (GitHub avertit au-delà de 50 Mo, mais l'accepte)" }

    $sha = (Get-FileHash $Dll -Algorithm SHA256).Hash.ToLower()
    $old = if (Test-Path "$out\libmpv-2.dll.sha256") { (Get-Content "$out\libmpv-2.dll.sha256" -Raw).Trim() } else { "" }
    if ($old -eq $sha) {
        Write-Host "Déjà cette DLL dans vendor\libmpv\windows-$name : rien à changer."
        return
    }
    New-Item -ItemType Directory -Force $out | Out-Null
    Move-Item -Force $gz "$out\libmpv-2.dll.gz"
    $utf8 = New-Object Text.UTF8Encoding($false)
    [IO.File]::WriteAllText("$out\libmpv-2.dll.sha256", "$sha`n", $utf8)
    [IO.File]::WriteAllText("$out\SOURCE.txt", "$Source`n", $utf8)
    Write-Host "vendor\libmpv\windows-$name mis à jour : $([math]::Round($size / 1MB)) Mo, SHA-256 $sha"
    Write-Host "Ensuite : cargo build (la nouvelle DLL est posée à côté de l'exécutable)."
}
finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
