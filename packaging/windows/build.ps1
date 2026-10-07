# Paquets Windows de Turtlefin : installeur (Inno Setup) et archive portable.
#
# Usage (depuis la racine du dépôt) :
#   powershell -ExecutionPolicy Bypass -File packaging\windows\build.ps1 -Arch x64
#   powershell -ExecutionPolicy Bypass -File packaging\windows\build.ps1 -Arch x86
# Prérequis : Rust (avec la cible i686-pc-windows-msvc pour x86), Inno Setup 6, 7-Zip, NASM (x86).
# Résultat dans target\dist : Turtlefin-<version>-windows-<arch>-setup.exe et -portable.zip.
param(
    [ValidateSet("x64", "x86")][string]$Arch = "x64"
)
$ErrorActionPreference = "Stop"
# CI : toute erreur est aussi écrite en annotation GitHub (lisible sans ouvrir le journal).
trap {
    if ($env:GITHUB_ACTIONS) { Write-Host "::error::build.ps1 ($Arch) : $($_.Exception.Message) [$($_.InvocationInfo.PositionMessage -replace '\s+', ' ')]" }
    break
}
$root = (Resolve-Path "$PSScriptRoot\..\..").Path
Set-Location $root

$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$target = if ($Arch -eq "x64") { "x86_64-pc-windows-msvc" } else { "i686-pc-windows-msvc" }
$mpvArch = if ($Arch -eq "x64") { "x86_64" } else { "i686" }
$dist = Join-Path $root "target\dist"
$stage = Join-Path $dist "Turtlefin-$Arch"
New-Item -ItemType Directory -Force $dist | Out-Null
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force $stage | Out-Null

# 0. NASM : nécessaire à la bibliothèque de chiffrement (aws-lc) en 32 bits.
foreach ($d in @("$env:LOCALAPPDATA\bin\NASM", "$env:ProgramFiles\NASM")) {
    if (Test-Path "$d\nasm.exe") { $env:PATH = "$d;$env:PATH" }
}

# 1. Compilation (TURTLEFIN_DIST : version publiée, mise à jour par GitHub Releases).
$env:TURTLEFIN_DIST = "release"
cargo build --release --target $target
if ($LASTEXITCODE -ne 0) { throw "échec de la compilation" }
Copy-Item "target\$target\release\turtlefin.exe" $stage

# 2. libmpv (builds de shinchiro/mpv-winbuild-cmake), gardée en cache dans target\mpv-<arch>.
$mpvDir = Join-Path $root "target\mpv-$mpvArch"
if (-not (Test-Path "$mpvDir\libmpv-2.dll")) {
    New-Item -ItemType Directory -Force $mpvDir | Out-Null
    # x86 : les versions 32 bits publiées depuis juillet 2026 plantent au démarrage (« OpenSSL
    # internal error: assertion failed: lock != NULL », mpv.exe seul compris) : on garde celle du
    # 10 juin 2026, la dernière qui fonctionne. MPV_TAG permet d'en imposer une autre.
    $tag = if ($env:MPV_TAG) { $env:MPV_TAG } elseif ($Arch -eq "x86") { "20260610" } else { "" }
    $api = if ($tag) { "tags/$tag" } else { "latest" }
    # Jeton de la CI s'il y en a un : les appels anonymes sont vite limités sur les machines partagées.
    $headers = @{ "User-Agent" = "turtlefin-build" }
    if ($env:GITHUB_TOKEN) { $headers["Authorization"] = "Bearer $env:GITHUB_TOKEN" }
    $rel = Invoke-RestMethod "https://api.github.com/repos/shinchiro/mpv-winbuild-cmake/releases/$api" -Headers $headers
    $asset = $rel.assets | Where-Object { $_.name -like "mpv-dev-$mpvArch-2*.7z" } | Select-Object -First 1
    if (-not $asset) { throw "libmpv $mpvArch introuvable" }
    $archive = Join-Path $mpvDir $asset.name
    Invoke-WebRequest $asset.browser_download_url -OutFile $archive -UseBasicParsing
    $7z = (Get-Command 7z -ErrorAction SilentlyContinue).Source
    if (-not $7z) { $7z = "C:\Program Files\7-Zip\7z.exe" }
    & $7z e $archive "-o$mpvDir" libmpv-2.dll -y | Out-Null
    if (-not (Test-Path "$mpvDir\libmpv-2.dll")) { throw "libmpv-2.dll absente de l'archive" }
}
Copy-Item "$mpvDir\libmpv-2.dll" $stage
Copy-Item README.md $stage

# 3. Installeur.
$iscc = @(
    (Get-Command iscc -ErrorAction SilentlyContinue).Source,
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
) | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
if (-not $iscc) { throw "Inno Setup 6 introuvable" }
& $iscc /Q "/DVersion=$version" "/DArch=$Arch" "/DSrc=$stage" "/DOut=$dist" "packaging\windows\turtlefin.iss"
if ($LASTEXITCODE -ne 0) { throw "échec de l'installeur" }

# 4. Archive portable : dossier « Turtlefin » avec le marqueur « portable ».
$port = Join-Path $dist "portable-$Arch"
if (Test-Path $port) { Remove-Item -Recurse -Force $port }
New-Item -ItemType Directory -Force "$port\Turtlefin" | Out-Null
Copy-Item "$stage\*" "$port\Turtlefin"
Copy-Item "packaging\windows\portable.txt" "$port\Turtlefin\portable"
$zip = Join-Path $dist "Turtlefin-$version-windows-$Arch-portable.zip"
if (Test-Path $zip) { Remove-Item $zip }
Compress-Archive -Path "$port\Turtlefin" -DestinationPath $zip

Write-Host "Paquets Windows $Arch :"
Get-ChildItem $dist -File -Filter "Turtlefin-$version-windows-$Arch-*" | ForEach-Object { "  {0} ({1:N1} Mo)" -f $_.Name, ($_.Length / 1MB) }
