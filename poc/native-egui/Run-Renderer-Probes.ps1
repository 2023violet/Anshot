# SR-V2-03 Windows renderer matrix. No screenshots/private images/network requests.
param(
  [string]$Exe = "poc/native-egui/target/release/shareready-native-egui-poc.exe",
  [string]$OutputDir = "poc/native-egui/evidence",
  [int]$TimeoutSeconds = 25
)
$ErrorActionPreference = "Stop"
if (-not (Test-Path $Exe)) { throw "EXE does not exist: $Exe" }
$exePath = (Resolve-Path $Exe).Path
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$dir = (Resolve-Path $OutputDir).Path
$system = @{
  os = [Environment]::OSVersion.VersionString
  runner = $env:RUNNER_NAME
  sha = $env:GITHUB_SHA
  exe_sha256 = (Get-FileHash $exePath -Algorithm SHA256).Hash.ToLowerInvariant()
}
try {
  $system.video_controllers = @(Get-CimInstance Win32_VideoController -ErrorAction Stop | ForEach-Object {
    @{ name=$_.Name; driver=$_.DriverVersion; status=$_.Status; resolution="$($_.CurrentHorizontalResolution)x$($_.CurrentVerticalResolution)" }
  })
} catch {
  $system.video_query_error = $_.Exception.Message
}
$results = @()
foreach ($case in @(
  @{ name="glow"; renderer="glow"; backend=$null },
  @{ name="wgpu_dx12"; renderer="wgpu"; backend="dx12" },
  @{ name="wgpu_auto"; renderer="wgpu"; backend=$null }
)) {
  $out = Join-Path $dir "$($case.name).stdout.txt"
  $err = Join-Path $dir "$($case.name).stderr.txt"
  Remove-Item -Path $out,$err -Force -ErrorAction SilentlyContinue
  $prior = $env:WGPU_BACKEND
  try {
    if ($null -eq $case.backend) { Remove-Item Env:WGPU_BACKEND -ErrorAction SilentlyContinue }
    else { $env:WGPU_BACKEND = $case.backend }
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $proc = Start-Process -FilePath $exePath -ArgumentList @("--renderer", $case.renderer, "--smoke-window") -PassThru -RedirectStandardOutput $out -RedirectStandardError $err
    $timedOut = $false
    while (-not $proc.HasExited -and $watch.Elapsed.TotalSeconds -lt $TimeoutSeconds) {
      Start-Sleep -Milliseconds 300
      $proc.Refresh()
    }
    if (-not $proc.HasExited) {
      $timedOut = $true
      Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
      $proc.WaitForExit()
    }
    $stdout = if (Test-Path $out) {Get-Content $out -Raw} else {""}
    $stderr = if (Test-Path $err) {Get-Content $err -Raw} else {""}
    $passed = (-not $timedOut) -and $proc.ExitCode -eq 0 -and $stdout.Contains("SR_V2_03_NATIVE_WINDOW_LIFECYCLE_PASS")
    $result = @{
      name=$case.name; renderer=$case.renderer; requested_backend=$case.backend
      passed=$passed; timed_out=$timedOut; exit_code=$proc.ExitCode
      wall_ms=[int]$watch.ElapsedMilliseconds
      adapter_line=@($stdout -split "\r?\n" | Where-Object {$_ -like "SR_V2_03_GPU_ADAPTER*" })
      stdout_file=[IO.Path]::GetFileName($out); stderr_file=[IO.Path]::GetFileName($err)
      error_excerpt=($stderr.Trim() | Select-Object -First 1)
    }
    $results += $result
    Write-Host "SR_V2_03_RENDERER_PROBE name=$($case.name) passed=$passed exit=$($proc.ExitCode) timeout=$timedOut"
    if ($stdout.Contains("SR_V2_03_GPU_ADAPTER")) {
      ($stdout -split "\r?\n" | Where-Object {$_ -like "SR_V2_03_GPU_ADAPTER*" }) | ForEach-Object { Write-Host $_ }
    }
    if (-not $passed -and $stderr.Trim()) {Write-Host "SR_V2_03_PROBE_ERROR $($case.name): $($stderr.Trim().Split([Environment]::NewLine)[0])"}
  } catch {
    $results += @{ name=$case.name; renderer=$case.renderer; requested_backend=$case.backend; passed=$false; error=$_.Exception.Message }
    Write-Host "SR_V2_03_RENDERER_PROBE name=$($case.name) passed=false error=$($_.Exception.Message)"
  } finally {
    if ($null -eq $prior) {Remove-Item Env:WGPU_BACKEND -ErrorAction SilentlyContinue}
    else {$env:WGPU_BACKEND = $prior}
  }
}
$report = @{
  schema="shareready.v2-03.renderer-matrix/1"
  system=$system
  results=$results
  limitations=@("CI Windows virtual machine does not validate physical mouse, multimonitor DPI, Paint paste or Narrator.",
    "D3D12 on Microsoft software adapter does not prove discrete GPU performance.",
    "One successful window lifecycle is not equivalent to G01-G10 human acceptance.")
}
$report | ConvertTo-Json -Depth 8 | Set-Content -Path (Join-Path $dir "renderer-matrix.json") -Encoding utf8
$passing = @($results | Where-Object { $_.passed }).Count
Write-Host "SR_V2_03_RENDERER_MATRIX_COMPLETE supported_runner=$passing/3"
if ($passing -eq 0) { exit 3 }
exit 0
