# Fabrique l'APK de Turtlefin pour Android.
#   powershell -ExecutionPolicy Bypass -File android\build-apk.ps1 [-Abi arm64-v8a,x86_64] [-Name arm64] [-Keystore cle.jks]
# Architectures : arm64-v8a (telephones 64 bits), armeabi-v7a (telephones 32 bits), x86_64 et x86 (emulateurs).
# Il faut : Android Studio (SDK, Java), le NDK (Android Studio > SDK Manager > NDK), Rust avec les cibles
# Android (rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
# i686-linux-android) et cargo-ndk (cargo install cargo-ndk).
# Resultat : target\dist\Turtlefin-<version>-android[-<Name>].apk (signe avec la cle de debogage
# d'Android Studio sans -Keystore).
param(
    [string[]]$Abi = @("arm64-v8a", "x86_64"),
    [string]$Name = "",
    [string]$Keystore = "",
    [string]$KeystorePass = "android",
    [string]$WorkDir = "$env:LOCALAPPDATA\turtlefin-android"
)
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
$version = (Select-String -Path "$PSScriptRoot\Cargo.toml" -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value

# Outils : SDK, NDK, Java d'Android Studio, plateforme et build-tools les plus recents qui soient complets.
$sdk = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { "$env:LOCALAPPDATA\Android\Sdk" }
$ndk = if ($env:ANDROID_NDK_HOME) { $env:ANDROID_NDK_HOME } else { (Get-ChildItem "$sdk\ndk" -Directory | Sort-Object Name | Select-Object -Last 1).FullName }
if (-not $ndk) { throw "NDK introuvable (Android Studio > SDK Manager > NDK)" }
if (-not $env:JAVA_HOME) { $env:JAVA_HOME = "$env:ProgramFiles\Android\Android Studio\jbr" }
$jar = (Get-ChildItem "$sdk\platforms\*\android.jar" | Sort-Object { [int]($_.Directory.Name -replace '\D', '') } | Select-Object -Last 1).FullName
$tools = (Get-ChildItem "$sdk\build-tools" -Directory | Where-Object { Test-Path "$($_.FullName)\aapt2.exe" } | Sort-Object { [version]$_.Name } | Select-Object -Last 1).FullName
$env:ANDROID_HOME = $sdk
$env:ANDROID_NDK_HOME = $ndk
$env:ANDROID_JAR = $jar
$env:ANDROID_BUILD_TOOLS_VERSION = Split-Path $tools -Leaf
# Compilation hors du depot (place) ; pages de 16 Ko acceptees (Android 15 et plus).
if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = "$WorkDir\target" }
$env:RUSTFLAGS = "-C link-arg=-Wl,-z,max-page-size=16384"
New-Item -ItemType Directory -Force $WorkDir | Out-Null

# 1. Turtlefin (une bibliotheque par architecture).
$targets = @{ "arm64-v8a" = "aarch64-linux-android"; "armeabi-v7a" = "armv7-linux-androideabi"; "x86_64" = "x86_64-linux-android"; "x86" = "i686-linux-android" }
Push-Location $PSScriptRoot
try {
    $ndkArgs = @()
    foreach ($a in $Abi) { $ndkArgs += @("-t", $a) }
    & cargo ndk @ndkArgs --platform 26 build --release
    if ($LASTEXITCODE -ne 0) { throw "echec de la compilation" }
} finally { Pop-Location }

# 2. libmpv pour Android (projet libmpv-android, Maven Central), empreinte verifiee.
$mpvVer = "1.0.0"
$mpvSha1 = "cc9acd422b2b23017e3760d34e74716dce49c9aa"
$aar = "$WorkDir\libmpv-$mpvVer.aar"
if (-not (Test-Path $aar)) {
    Invoke-WebRequest "https://repo1.maven.org/maven2/dev/jdtech/mpv/libmpv/$mpvVer/libmpv-$mpvVer.aar" -OutFile $aar -UseBasicParsing
}
if ((Get-FileHash $aar -Algorithm SHA1).Hash.ToLower() -ne $mpvSha1) { Remove-Item $aar; throw "libmpv : empreinte differente de celle attendue" }
$mpvDir = "$WorkDir\libmpv-$mpvVer"
if (-not (Test-Path $mpvDir)) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::ExtractToDirectory($aar, $mpvDir)
}

# 3. Ressources (icone) et manifeste.
$build = "$WorkDir\apk"
Remove-Item $build -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force "$build\res\mipmap-xxxhdpi" | Out-Null
Copy-Item "$root\packaging\icons\turtlefin-256.png" "$build\res\mipmap-xxxhdpi\ic_launcher.png"
& "$tools\aapt2.exe" compile --dir "$build\res" -o "$build\res.zip"
if ($LASTEXITCODE -ne 0) { throw "aapt2 compile" }
$code = ($version.Split('.') | ForEach-Object { [int]$_ })
$versionCode = $code[0] * 10000 + $code[1] * 100 + $code[2]
& "$tools\aapt2.exe" link -o "$build\base.apk" -I $jar --manifest "$PSScriptRoot\AndroidManifest.xml" "$build\res.zip" `
    --min-sdk-version 26 --target-sdk-version 34 --version-code $versionCode --version-name $version
if ($LASTEXITCODE -ne 0) { throw "aapt2 link" }

# 4. Bibliotheques : Turtlefin + libmpv et ses dependances (pas libplayer.so, l'enveloppe Java de libmpv).
Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::Open("$build\base.apk", "Update")
try {
    foreach ($a in $Abi) {
        $so = "$env:CARGO_TARGET_DIR\$($targets[$a])\release\libturtlefin.so"
        [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $so, "lib/$a/libturtlefin.so") | Out-Null
        foreach ($f in Get-ChildItem "$mpvDir\jni\$a\*.so" | Where-Object { $_.Name -ne "libplayer.so" }) {
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $f.FullName, "lib/$a/$($f.Name)") | Out-Null
        }
    }
} finally { $zip.Dispose() }

# 5. Alignement puis signature.
& "$tools\zipalign.exe" -p -f 4 "$build\base.apk" "$build\aligned.apk"
if ($LASTEXITCODE -ne 0) { throw "zipalign" }
if (-not $Keystore) { $Keystore = "$env:USERPROFILE\.android\debug.keystore" }
New-Item -ItemType Directory -Force "$root\target\dist" | Out-Null
$suffix = if ($Name) { "-$Name" } else { "" }
$out = "$root\target\dist\Turtlefin-$version-android$suffix.apk"
& "$tools\apksigner.bat" sign --ks $Keystore --ks-pass "pass:$KeystorePass" --out $out "$build\aligned.apk"
if ($LASTEXITCODE -ne 0) { throw "apksigner" }
Write-Host "APK : $out"
