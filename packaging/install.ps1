# Installation de Turtlefin en une commande (Windows) :
#   irm https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.ps1 | iex
#
# Télécharge l'installeur de la dernière version publiée (x64 ou x86 selon Windows) et l'exécute sans
# questions, pour l'utilisateur en cours (pas de droits administrateur). Langue : celle de Windows.
# Turtlefin se lance à la fin. Messages en anglais : le script sert à tout le monde.
$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$arch = if ([Environment]::Is64BitOperatingSystem) { 'x64' } else { 'x86' }
Write-Host "Turtlefin: looking for the latest version ($arch)..."
$release = Invoke-RestMethod 'https://api.github.com/repos/Xelopteryx/Turtlefin/releases/latest' -Headers @{ 'User-Agent' = 'turtlefin-install' }
$asset = $release.assets | Where-Object { $_.name -like "*-windows-$arch-setup.exe" } | Select-Object -First 1
if (-not $asset) { throw "No Windows $arch installer in release $($release.tag_name)." }

$file = Join-Path $env:TEMP $asset.name
Write-Host "Downloading $($asset.name)..."
Invoke-WebRequest $asset.browser_download_url -OutFile $file -UseBasicParsing
Write-Host 'Installing...'
# WaitForExit plutôt que -Wait : -Wait attendrait aussi Turtlefin, lancé par l'installeur à la fin.
# Langue : celle de Windows si Turtlefin la connaît, sinon l'anglais (elle devient celle de l'appli).
$lang = (Get-UICulture).TwoLetterISOLanguageName
if ($lang -notin 'en', 'fr', 'es', 'de', 'it', 'pt', 'pl', 'nl') { $lang = 'en' }
(Start-Process $file -ArgumentList '/SILENT', '/SUPPRESSMSGBOXES', '/CURRENTUSER', '/NORESTART', "/LANG=$lang" -PassThru).WaitForExit()
Remove-Item $file -ErrorAction SilentlyContinue
Write-Host "Turtlefin $($release.tag_name) is installed (Start menu)."
