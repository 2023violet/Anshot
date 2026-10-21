# Build the local, unsigned Windows release files of Anshot (安截) into dist/:
#   dist/Anshot-<v>-win-x64/                    portable folder (also zipped)
#   dist/Anshot-<v>-win-x64.zip                 portable package
#   dist/Anshot-<v>-win-x64-setup.exe           per-user NSIS installer
#   dist/SHA256SUMS.txt, dist/release-metadata.json
# Usage: pwsh -File scripts/package.ps1   (NSIS 3 makensis: $env:MAKENSIS, PATH, or the Tauri-cached copy)
# Nothing is uploaded or published; the files are not code-signed.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$version = (Select-String -Path "$root\apps\desktop\Cargo.toml" -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
$name = "Anshot-$version-win-x64"
$out = Join-Path $root 'dist'
$dist = Join-Path $out $name

cargo build --release --locked --manifest-path "$root\apps\desktop\Cargo.toml"
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { "$root\apps\desktop\target" }

if (Test-Path $dist) { Get-ChildItem $dist | ForEach-Object { [System.IO.File]::Delete($_.FullName) } } else { New-Item -ItemType Directory -Force $dist | Out-Null }
Copy-Item "$target\release\anshot.exe" "$dist\Anshot.exe"
Copy-Item "$root\apps\desktop\assets\icon.ico" "$dist\anshot.ico"
Copy-Item "$root\LICENSE" "$dist\LICENSE.txt"
Copy-Item "$root\apps\desktop\THIRD_PARTY_LICENSES.md" "$dist\THIRD_PARTY_LICENSES.md"
Copy-Item "$root\assets\fonts\OFL-1.1.txt" "$dist\FONT-OFL-1.1.txt"
@"
安截 Anshot $version — Windows 截图、标注、遮挡后分享

运行：双击 Anshot.exe（便携版），或运行安装程序。程序常驻托盘；按 Ctrl+Shift+A 截图，
      或打开 / 粘贴 / 拖入 PNG、JPEG。
截图：拖出选区后可直接 Ctrl+C 复制、Ctrl+S 保存、Ctrl+T 钉到桌面，或点“长截图”后滚动内容拼接。
编辑：V 选择 · A 箭头 / L 直线 · R 框 / E 椭圆 · P 画笔 · T 文字 · N 序号 · H 高亮 ·
      B 实心遮挡 · M 安全马赛克 · C 裁剪；1–6 选色，7 自定义色；按住 Shift 画正方形 / 45°。
交付：Ctrl+C 复制，Ctrl+S 另存为新 PNG（不覆盖已有文件）；Ctrl+H 截图历史。
设置：Ctrl+, 或托盘“设置…”：截图快捷键、复制后是否收起、截图历史张数、默认保存位置、开机启动。

隐私：图片只在本机处理，程序不联网、无账号、无遥测。遮挡区域导出为不含原图信息的像素；
      请自行检查未遮挡的内容。截图历史保存编辑器复制或保存过的图片（与分享出去的一致，本机未加密，可在设置中关闭）。
签名：本程序未做代码签名，首次运行 Windows 可能提示“已保护你的电脑”，可点“更多信息 → 仍要运行”；
      请用 SHA256SUMS.txt 核对下载文件。未经第三方安全审计。

许可：MIT（LICENSE.txt）。界面使用 Slint（Royalty-free 2.0，见“关于”）；字体 Noto Sans CJK SC（OFL-1.1）。
"@ | Set-Content -Encoding utf8 "$dist\README.txt"

# Portable zip.
$zip = Join-Path $out "$name.zip"
if (Test-Path $zip) { [System.IO.File]::Delete($zip) }
Compress-Archive -Path "$dist\*" -DestinationPath $zip

# Installer.
$makensis = @($env:MAKENSIS, (Get-Command makensis -ErrorAction SilentlyContinue).Source, "$env:LOCALAPPDATA\tauri\NSIS\makensis.exe") |
    Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
if (-not $makensis) { throw "makensis (NSIS 3) not found; set `$env:MAKENSIS" }
$setup = Join-Path $out "$name-setup.exe"
if (Test-Path $setup) { [System.IO.File]::Delete($setup) }
& $makensis /V2 "/DVERSION=$version" "/DSRC=$dist" "/DOUT=$setup" "$root\installer\anshot.nsi"
if ($LASTEXITCODE -ne 0) { throw "makensis failed" }

# Checksums and honest metadata for the release files.
$assets = @($setup, $zip)
$sums = $assets | ForEach-Object { "$((Get-FileHash $_ -Algorithm SHA256).Hash.ToLower())  $(Split-Path $_ -Leaf)" }
# LF line endings so `sha256sum -c` works on any platform.
[System.IO.File]::WriteAllText((Join-Path $out 'SHA256SUMS.txt'), (($sums -join "`n") + "`n"), [System.Text.Encoding]::ASCII)
$exeHash = (Get-FileHash "$dist\Anshot.exe" -Algorithm SHA256).Hash.ToLower()
$meta = [ordered]@{
    name = 'Anshot (安截)'; version = $version; platform = 'windows-x64'
    commit = (git -C $root rev-parse HEAD); dirty = [bool](git -C $root status --porcelain)
    signing = 'unsigned'; updater = 'manual (About -> 检查更新 opens the releases page; the app makes no network requests)'
    install_scope = 'per-user (%LOCALAPPDATA%\Programs\Anshot), no administrator rights'
    exe_sha256 = $exeHash
    files = @($assets | ForEach-Object { [ordered]@{ name = (Split-Path $_ -Leaf); sha256 = (Get-FileHash $_ -Algorithm SHA256).Hash.ToLower(); bytes = (Get-Item $_).Length } })
    toolchain = [ordered]@{ rustc = (rustc --version); cargo = (cargo --version); makensis = (& $makensis /VERSION) }
    build = 'pwsh -File scripts/package.ps1 (cargo build --release --locked --manifest-path apps/desktop/Cargo.toml)'
}
$meta | ConvertTo-Json -Depth 4 | Set-Content -Encoding utf8 (Join-Path $out 'release-metadata.json')
"PACKAGED $zip"
"INSTALLER $setup"
"EXE $exeHash"
$sums
