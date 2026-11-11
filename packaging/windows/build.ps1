# Paquets Windows de Turtlefin : installeur (Inno Setup) et archive portable.
#
# Usage (depuis la racine du dépôt) :
#   powershell -ExecutionPolicy Bypass -File packaging\windows\build.ps1 -Arch x64
#   powershell -ExecutionPolicy Bypass -File packaging\windows\build.ps1 -Arch x86
# Prérequis : Rust (avec la cible i686-pc-windows-msvc pour x86), Inno Setup 6, 7-Zip, NASM (x86).
# libmpv : dans le dépôt (vendor\libmpv), posée par build.rs.
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

# 2. libmpv : fournie par le dépôt (vendor\libmpv), décompressée et vérifiée par build.rs à côté
# de l'exécutable pendant la compilation (rien à télécharger).
$mpvDll = "target\$target\release\libmpv-2.dll"
if (-not (Test-Path $mpvDll)) { throw "libmpv-2.dll absente de target\$target\release (voir vendor\libmpv\README.md)" }
Copy-Item $mpvDll $stage
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
