# Installs the stargazer CLI from GitHub Releases (Windows):
#   irm https://raw.githubusercontent.com/Mhon-Cloud/stargazer-cli/main/install.ps1 | iex
# $env:STARGAZER_VERSION = "v0.1.0" picks a release (default: the latest); $env:STARGAZER_INSTALL_DIR the folder
# (default: %LOCALAPPDATA%\Programs\stargazer, added to your user PATH). The SHA-256 is checked first.
# Uninstall (the binary, its PATH entry and the saved key / config):
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/Mhon-Cloud/stargazer-cli/main/install.ps1))) -Uninstall
param([switch]$Uninstall)
$ErrorActionPreference = "Stop"

$repo = "Mhon-Cloud/stargazer-cli"
$version = if ($env:STARGAZER_VERSION) { $env:STARGAZER_VERSION } else { "latest" }
$installDir = if ($env:STARGAZER_INSTALL_DIR) { $env:STARGAZER_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "Programs\stargazer" }

if ($Uninstall) {
    if (Test-Path $installDir) { Remove-Item -Recurse -Force $installDir; Write-Host "Removed $installDir" } else { Write-Host "No stargazer at $installDir" }
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $kept = ($userPath -split ';' | Where-Object { $_ -and $_ -ne $installDir }) -join ';'
    if ($kept -ne $userPath) { [Environment]::SetEnvironmentVariable("Path", $kept, "User"); Write-Host "Removed it from your PATH" }
    $config = if ($env:STARGAZER_CONFIG_DIR) { $env:STARGAZER_CONFIG_DIR } else { Join-Path $env:APPDATA "stargazer" }
    if (Test-Path $config) { Remove-Item -Recurse -Force $config; Write-Host "Removed $config (your saved API key)" }
    Write-Host "Done. Keys stay valid until you revoke them in the console (Inventory > Additional Services > CLI)."
    return
}

$asset = "stargazer-x86_64-pc-windows-msvc.zip"
$base = if ($version -eq "latest") { "https://github.com/$repo/releases/latest/download" } else { "https://github.com/$repo/releases/download/$version" }

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
    Write-Host "Downloading $asset ($version)..."
    Invoke-WebRequest -UseBasicParsing "$base/$asset" -OutFile (Join-Path $tmp $asset)
    Invoke-WebRequest -UseBasicParsing "$base/$asset.sha256" -OutFile (Join-Path $tmp "$asset.sha256")

    $expected = ((Get-Content (Join-Path $tmp "$asset.sha256") -Raw).Trim() -split '\s+')[0].ToLower()
    $actual = (Get-FileHash (Join-Path $tmp $asset) -Algorithm SHA256).Hash.ToLower()
    if ($expected -ne $actual) { throw "checksum mismatch for $asset; nothing was installed" }

    New-Item -ItemType Directory -Force -Path $installDir | Out-Null
    Expand-Archive -Path (Join-Path $tmp $asset) -DestinationPath $installDir -Force

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (-not ($userPath -split ';' | Where-Object { $_ -eq $installDir })) {
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
        Write-Host "Added $installDir to your PATH (open a new terminal to use it)."
    }
    & (Join-Path $installDir "stargazer.exe") version --output table
    Write-Host "Next: stargazer login   (create a key in the console: Inventory > Additional Services > CLI)"
}
finally {
    Remove-Item -Recurse -Force $tmp
}
