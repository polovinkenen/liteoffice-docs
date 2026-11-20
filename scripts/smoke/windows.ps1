param(
  [Parameter(Position = 0)]
  [string]$Artifacts = "artifacts"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$appData = Join-Path $env:APPDATA "app.liteoffice.docs"
$recent = Join-Path $appData "recent.json"

function Get-InstallRoots {
  $roots = @(
    (Join-Path $env:LOCALAPPDATA "LiteOffice Docs"),
    (Join-Path $env:ProgramFiles "LiteOffice Docs")
  )

  if (${env:ProgramFiles(x86)}) {
    $roots += Join-Path ${env:ProgramFiles(x86)} "LiteOffice Docs"
  }

  return $roots
}

function Find-AppExe {
  foreach ($root in Get-InstallRoots) {
    $exe = Join-Path $root "liteoffice-docs.exe"
    if (Test-Path $exe) {
      return $exe
    }
  }
  return $null
}

function Wait-AppExe([int]$seconds = 60) {
  for ($i = 0; $i -lt $seconds; $i++) {
    $exe = Find-AppExe
    if ($exe) {
      return $exe
    }
    Start-Sleep -Seconds 1
  }
  return $null
}

function Wait-NoExe([string]$path, [int]$seconds = 30) {
  for ($i = 0; $i -lt $seconds; $i++) {
    if (-not (Test-Path $path)) {
      return
    }
    Start-Sleep -Seconds 1
  }
  throw "после удаления остался $path"
}

function Get-AssociationProgIds([string]$root, [string]$extension) {
  $key = "Registry::$root\Software\Classes\$extension"
  if (-not (Test-Path $key)) {
    return @()
  }

  $ids = @()
  $default = (Get-Item -Path $key).GetValue("")
  if ($default) {
    $ids += $default
  }

  $openWith = Join-Path $key "OpenWithProgids"
  if (Test-Path $openWith) {
    $ids += (Get-Item -Path $openWith).GetValueNames()
  }

  return $ids | Where-Object { $_ } | Select-Object -Unique
}

function Assert-Association([string]$extension, [string]$exe) {
  foreach ($root in @("HKEY_CURRENT_USER", "HKEY_LOCAL_MACHINE")) {
    foreach ($id in Get-AssociationProgIds $root $extension) {
      $commandKey = "Registry::$root\Software\Classes\$id\shell\open\command"
      if (-not (Test-Path $commandKey)) {
        continue
      }
      $command = (Get-Item -Path $commandKey).GetValue("")
      if ($command -and $command -like "*$exe*") {
        Write-Host "ассоциация .${extension}: $id"
        return
      }
    }
  }

  throw "команда открытия .$extension не указывает на $exe"
}

function Clear-AppData {
  Remove-Item -Path $appData -Recurse -Force -ErrorAction SilentlyContinue
}

function Wait-Recent([string]$needle, [int]$seconds = 60) {
  for ($i = 0; $i -lt $seconds; $i++) {
    if (Test-Path $recent) {
      $text = Get-Content -Raw -Path $recent
      if ($text -like "*$needle*") {
        return
      }
    }
    Start-Sleep -Seconds 1
  }

  if (Test-Path $recent) {
    Write-Host (Get-Content -Raw -Path $recent)
  }
  throw "не дождались открытия $needle"
}

function Stop-App {
  Get-Process -Name "liteoffice-docs" -ErrorAction SilentlyContinue | Stop-Process -Force
}

function Invoke-OpenCheck([string]$exe) {
  Clear-AppData
  $work = Join-Path $env:RUNNER_TEMP ([guid]::NewGuid().ToString("N"))
  New-Item -ItemType Directory -Path $work | Out-Null

  $first = Join-Path $work "первый.md"
  Set-Content -Path $first -Value "# Первый" -Encoding utf8
  $firstProcess = Start-Process -FilePath $exe -ArgumentList "`"$first`"" -PassThru

  Wait-Recent (Split-Path $first -Leaf)

  if ($firstProcess.HasExited) {
    throw "приложение завершилось после открытия первого файла"
  }

  $second = Join-Path $work "второй.md"
  Set-Content -Path $second -Value "# Второй" -Encoding utf8
  $secondProcess = Start-Process -FilePath $exe -ArgumentList "`"$second`"" -PassThru
  if (-not $secondProcess.WaitForExit(30000)) {
    throw "второй экземпляр не завершился после передачи пути"
  }

  Wait-Recent (Split-Path $second -Leaf)

  Stop-Process -Id $firstProcess.Id -Force -ErrorAction SilentlyContinue
  Stop-App
  Start-Sleep -Seconds 2
  Write-Host "e2e: $exe открыл оба файла"
}

$nsis = Get-ChildItem -Path $Artifacts -Recurse -Filter "*.exe" |
  Where-Object { $_.Name -notlike "*uninstall*" } |
  Select-Object -First 1
$msi = Get-ChildItem -Path $Artifacts -Recurse -Filter "*.msi" | Select-Object -First 1

if (-not $nsis -or -not $msi) {
  throw "не найдены установщики NSIS и MSI в $Artifacts"
}

Write-Host "NSIS: $($nsis.FullName)"
$nsisProcess = Start-Process -FilePath $nsis.FullName -ArgumentList "/S" -Wait -PassThru
if ($nsisProcess.ExitCode -ne 0) {
  throw "установка NSIS завершилась с кодом $($nsisProcess.ExitCode)"
}

$exe = Wait-AppExe
if (-not $exe) {
  throw "после установки NSIS не найден liteoffice-docs.exe"
}

Assert-Association "md" $exe
Assert-Association "docx" $exe
Invoke-OpenCheck $exe

$uninstaller = Get-ChildItem -Path (Split-Path $exe) -Filter "uninstall*.exe" | Select-Object -First 1
if (-not $uninstaller) {
  throw "не найден деинсталлятор NSIS"
}
$uninstallProcess = Start-Process -FilePath $uninstaller.FullName -ArgumentList "/S" -Wait -PassThru
if ($uninstallProcess.ExitCode -ne 0) {
  throw "удаление NSIS завершилось с кодом $($uninstallProcess.ExitCode)"
}
Wait-NoExe $exe

Write-Host "NSIS: установка, ассоциации, открытие и удаление — ok"

$msiProcess = Start-Process -FilePath "msiexec.exe" `
  -ArgumentList "/i", "`"$($msi.FullName)`"", "/qn", "/norestart" `
  -Wait -PassThru
if ($msiProcess.ExitCode -ne 0) {
  throw "установка MSI завершилась с кодом $($msiProcess.ExitCode)"
}

$exe = Wait-AppExe
if (-not $exe) {
  throw "после установки MSI не найден liteoffice-docs.exe"
}

Assert-Association "md" $exe

$uninstallRoots = @(
  "Registry::HKEY_LOCAL_MACHINE\Software\Microsoft\Windows\CurrentVersion\Uninstall",
  "Registry::HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Uninstall"
)
$entry = Get-ChildItem -Path $uninstallRoots -ErrorAction SilentlyContinue |
  Get-ItemProperty |
  Where-Object { $_.DisplayName -like "LiteOffice Docs*" } |
  Select-Object -First 1
if (-not $entry) {
  throw "MSI не оставил запись деинсталляции"
}

$code = Split-Path $entry.PSPath -Leaf
$removeProcess = Start-Process -FilePath "msiexec.exe" `
  -ArgumentList "/x", $code, "/qn", "/norestart" `
  -Wait -PassThru
if ($removeProcess.ExitCode -ne 0) {
  throw "удаление MSI завершилось с кодом $($removeProcess.ExitCode)"
}
Wait-NoExe $exe

Write-Host "MSI: установка, ассоциации и удаление — ok"
Write-Host "smoke windows: ok"
