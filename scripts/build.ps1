param(
    [string]$BuildRoot = (Join-Path $env:LOCALAPPDATA 'OpenProjectManager-build'),
    [string]$SourceRoot = (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent)
)
$ErrorActionPreference = 'Stop'
$manager = Split-Path $PSScriptRoot -Parent
$source = [IO.Path]::GetFullPath($SourceRoot)
if(!(Test-Path (Join-Path $source 'Gemfile'))){throw 'OpenProject source is missing. Run scripts/prepare-source.ps1, then pass its path as -SourceRoot.'}
$BuildRoot = [IO.Path]::GetFullPath($BuildRoot)
$stage = Join-Path $BuildRoot 'bundle'
$probe = Join-Path $env:LOCALAPPDATA 'OpenProject-native-probe'
New-Item -ItemType Directory -Force -Path $stage | Out-Null

function Copy-Tree([string]$From,[string]$To) {
    if(!(Test-Path -LiteralPath $From -PathType Container)){throw "Build prerequisite missing: $From"}
    & robocopy.exe $From $To /E /R:1 /W:1 /NFL /NDL /NJH /NJS /NP | Out-Null
    if($LASTEXITCODE -gt 7){throw "Copy failed: $From"}
}

$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$previousTarget = $env:CARGO_TARGET_DIR
if(!$env:CARGO_TARGET_DIR){$env:CARGO_TARGET_DIR=Join-Path $BuildRoot 'target'}
Push-Location $manager
try {
    & $cargo test --locked --release --lib --no-default-features -j 2
    if($LASTEXITCODE -ne 0){throw 'Rust backend tests failed'}
    & $cargo build --locked --release -j 2
    if($LASTEXITCODE -ne 0){throw 'Rust manager build failed'}
    & $cargo test --locked --release --bin openproject-manager -j 2
    if($LASTEXITCODE -ne 0){throw 'Manager resource tests failed'}
    & $cargo test --locked --release -p gpui-pre-windows --lib server_2016 -j 2
    if($LASTEXITCODE -ne 0){throw 'Server 2016 GPUI compatibility tests failed'}

    $exe=Join-Path $env:CARGO_TARGET_DIR 'release\openproject-manager.exe'
    $vswhere=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    $dumpbin=& $vswhere -latest -prerelease -products '*' -find 'VC\Tools\MSVC\**\bin\Hostx64\x64\dumpbin.exe' | Select-Object -First 1
    if(!$dumpbin){throw 'Visual Studio C++ Build Tools are required'}
    $imports=& $dumpbin /imports $exe
    if($LASTEXITCODE -ne 0 -or $imports -match '^\s+icu(?:uc|in)?\.dll\s*$'){throw 'Manager import check failed; retain the Server 2016 GPUI patch'}

    $app=Join-Path $stage 'openproject'
    $roots=@('app','bin','config','db','extensions','frontend','lib','lib_static','modules','public','vendor','vendored-plugins')
    $rootFiles=@('Gemfile','Gemfile.lock','Gemfile.modules','.ruby-version','.openproject-token.pub','Rakefile','config.ru','COPYRIGHT','COPYRIGHT_short','LICENSE','README.md','package.json','package-lock.json','.npmrc')
    Push-Location $source
    try {
        $files = git ls-files
        if($LASTEXITCODE -ne 0){throw 'Cannot enumerate OpenProject source'}
        foreach($relative in $files) {
            $top=($relative -split '/')[0]
            if($rootFiles -contains $relative -or $roots -contains $top) {
                $destination=Join-Path $app $relative
                New-Item -ItemType Directory -Force -Path (Split-Path $destination) | Out-Null
                Copy-Item -LiteralPath (Join-Path $source $relative) -Destination $destination -Force
            }
        }
    } finally { Pop-Location }
    Copy-Tree (Join-Path $source 'public\assets') (Join-Path $app 'public\assets')
    Copy-Tree (Join-Path $source 'public\styles') (Join-Path $app 'public\styles')
    Copy-Tree (Join-Path $source 'public\fonts\inter') (Join-Path $app 'public\fonts\inter')
    New-Item -ItemType Directory -Force -Path (Join-Path $app 'tmp'),(Join-Path $app 'log'),(Join-Path $app 'script\windows') | Out-Null
    Copy-Item -LiteralPath (Join-Path $source 'script\windows\check.rb') -Destination (Join-Path $app 'script\windows\check.rb') -Force
    Copy-Item -LiteralPath (Join-Path $source 'script\windows\test_file_store.rb') -Destination (Join-Path $app 'script\windows\test_file_store.rb') -Force
    Copy-Item -LiteralPath (Join-Path $source 'config\initializers\windows_file_store.rb') -Destination (Join-Path $app 'config\initializers\windows_file_store.rb') -Force
    Copy-Item -LiteralPath (Join-Path $source 'config\database.production.yml') -Destination (Join-Path $app 'config\database.yml') -Force
    Copy-Item -LiteralPath (Join-Path $source 'config\frontend_assets.manifest.json') -Destination (Join-Path $app 'config\frontend_assets.manifest.json') -Force
    Copy-Tree (Join-Path $probe 'ruby\rubyinstaller-4.0.6-1-x64') (Join-Path $stage 'runtime\ruby')
    Copy-Tree (Join-Path $probe 'gems') (Join-Path $stage 'runtime\gems')
    Copy-Tree (Join-Path $probe 'postgresql\pgsql') (Join-Path $stage 'runtime\postgresql')

    $tools=Join-Path $manager 'tools'
    New-Item -ItemType Directory -Force -Path $tools | Out-Null
    $winsw=Join-Path $tools 'winsw.exe'
    if(!(Test-Path $winsw)){Invoke-WebRequest 'https://github.com/winsw/winsw/releases/download/v2.12.0/WinSW.NET461.exe' -OutFile $winsw}
    if((Get-FileHash $winsw -Algorithm SHA256).Hash -ne 'B5066B7BBDFBA1293E5D15CDA3CAAEA88FBEAB35BD5B38C41C913D492AADFC4F'){throw 'WinSW checksum mismatch'}
    New-Item -ItemType Directory -Force -Path (Join-Path $stage 'services'),(Join-Path $stage 'licenses\gpui-pre-windows') | Out-Null
    Copy-Item -LiteralPath $winsw -Destination (Join-Path $stage 'services\winsw.exe') -Force
    $license=Join-Path $tools 'WinSW-LICENSE.txt'
    if(!(Test-Path $license)){Invoke-WebRequest 'https://raw.githubusercontent.com/winsw/winsw/v2.12.0/LICENSE.txt' -OutFile $license}
    Copy-Item -LiteralPath $license -Destination (Join-Path $stage 'licenses\WinSW-LICENSE.txt') -Force
    Copy-Item -LiteralPath (Join-Path $manager 'LICENSE') -Destination (Join-Path $stage 'licenses\OpenProjectManager-LICENSE.txt') -Force
    foreach($file in 'LICENSE-APACHE','PATCH.md'){Copy-Item -LiteralPath (Join-Path $manager "vendor\gpui-pre-windows\$file") -Destination (Join-Path $stage 'licenses\gpui-pre-windows') -Force}
    foreach($file in 'setup-native.ps1','service.rb'){Copy-Item -LiteralPath (Join-Path $PSScriptRoot $file) -Destination $stage -Force}
    Copy-Item -LiteralPath (Join-Path $manager 'README.md') -Destination $stage -Force
    Copy-Item -LiteralPath (Join-Path $source 'BOARDUI-THEME.md') -Destination $stage -Force

    $crt=& $vswhere -latest -prerelease -products '*' -find 'VC\Redist\MSVC\**\x64\Microsoft.VC*.CRT\vcruntime140.dll' | Where-Object {$_ -notmatch '\\onecore\\'} | Select-Object -First 1
    if(!$crt){throw 'Visual C++ redistributable DLLs are required'}
    foreach($destination in @($stage,(Join-Path $stage 'runtime\ruby\bin'),(Join-Path $stage 'runtime\postgresql\bin'))) {
        Get-ChildItem -LiteralPath (Split-Path $crt) -Filter '*.dll' | Copy-Item -Destination $destination -Force
    }

    & powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File (Join-Path $stage 'setup-native.ps1') -Root $stage -CheckBundle
    if($LASTEXITCODE -ne 0){throw 'Relocated native bundle verification failed'}
    & "$PSScriptRoot\test-setup.ps1" -Root $stage
    & "$PSScriptRoot\test-native-bundle.ps1" -Root $stage -BuildRoot $BuildRoot
    $iscc=(Get-Command ISCC.exe -ErrorAction Stop).Source
    $output=Join-Path $BuildRoot 'dist'
    & $iscc /Q "/O$output" "/DManagerSource=$exe" "/DBundleSource=$stage" (Join-Path $manager 'installer\openproject-manager.iss')
    if($LASTEXITCODE -ne 0){throw 'Installer compilation failed'}
    $installer=Join-Path $output 'OpenProjectManager-Setup.exe'
    $localOutput=Join-Path $manager 'dist'
    New-Item -ItemType Directory -Force -Path $localOutput | Out-Null
    if((Get-Item $installer).Length -lt ([IO.DriveInfo]::new([IO.Path]::GetPathRoot($localOutput))).AvailableFreeSpace - 100MB) {
        Copy-Item -LiteralPath $installer -Destination $localOutput -Force
        $installer=Join-Path $localOutput 'OpenProjectManager-Setup.exe'
    }
    Write-Output "Built installer: $installer"
    Get-FileHash -LiteralPath $installer -Algorithm SHA256
} finally {
    Pop-Location
    $env:CARGO_TARGET_DIR=$previousTarget
}
