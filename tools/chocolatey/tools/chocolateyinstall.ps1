# Chocolatey install script for Windows

$ErrorActionPreference = 'Stop'
$ToolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"
$Url64 = 'https://github.com/sentinel-team/sentinel-pro/releases/download/v5.0.0-pro.beta.3/sentinel-pro-5.0.0-pro.beta.3-x86_64-pc-windows-msvc.zip'
$Checksum64 = 'PLACEHOLDER_CHECKSUM'

$InstallDir = "$(Join-Path $env:ALLUSERSPROFILE 'Sentinel')"

if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$ZipFile = Join-Path $env:TEMP 'sentinel-pro.zip'
Get-ChocolateyWebFile -PackageName 'sentinel-pro' -FileFullPath $ZipFile -Url64bit $Url64 -Checksum64 $Checksum64 -ChecksumType64 'sha256'

Get-ChocolateyUnzip -FileFullPath $ZipFile -Destination $InstallDir

Install-ChocolateyPath $InstallDir
