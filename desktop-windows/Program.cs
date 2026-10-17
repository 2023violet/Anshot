using System.Diagnostics;
using System.Text.Json;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;

namespace ShareReady.Desktop;

internal static class Program
{
    [STAThread]
    private static void Main(string[] args)
    {
        if (args.Length == 1 && args[0] == "--verify-package")
        {
            Environment.ExitCode = ShareReadyWindow.VerifyPackage() ? 0 : 3;
            return;
        }
        var smoke = args.Length == 1 && args[0] == "--smoke-native";
        if (args.Length != 0 && !smoke)
        {
            Environment.ExitCode = 2;
            return;
        }
        ApplicationConfiguration.Initialize();
        Application.Run(new ShareReadyWindow(smoke));
    }
}

/// <summary>
/// Windows-native window for the bundled Rust/WASM editor.
/// Files are not uploaded: WebView2 resolves a local virtual origin without
/// opening an HTTP port; only explicit user-initiated PNG downloads may write.
/// </summary>
internal sealed class ShareReadyWindow : Form
{
    private const string AppUrl = "https://shareready.local/index.html";
    private const string AppHost = "shareready.local";
    private readonly WebView2 editor = new()
    {
        Dock = DockStyle.Fill,
        AllowExternalDrop = true
    };
    private readonly bool smokeMode;
    private string? sessionFolder;
    private string? smokeOutput;
    private bool smokeCancelNextDownload;
    private bool webReady;

    internal ShareReadyWindow(bool smokeMode = false)
    {
        this.smokeMode = smokeMode;
        Text = "ShareReady — 分享前隐私遮挡";
        MinimumSize = new Size(920, 640);
        Size = new Size(1320, 860);
        StartPosition = FormStartPosition.CenterScreen;
        BackColor = Color.FromArgb(248, 250, 252);
        AutoScaleMode = AutoScaleMode.Dpi;
        Controls.Add(editor);
        Shown += async (_, _) => await InitializeEditorAsync();
        FormClosed += (_, _) => CleanupSession();
    }

    private static string WebRoot => Path.Combine(AppContext.BaseDirectory, "www");
    internal static bool VerifyPackage()
    {
        foreach (var asset in new[]
        {
            "index.html", "app.js", "pkg/shareready_e1.js",
            "pkg/shareready_e1_bg.wasm"
        })
        {
            var file = Path.Combine(WebRoot, asset.Replace('/', Path.DirectorySeparatorChar));
            if (!File.Exists(file) || new FileInfo(file).Length == 0)
            {
                Console.Error.WriteLine("MISSING_LOCAL_ASSET: " + asset);
                return false;
            }
        }
        return true;
    }

    private async Task InitializeEditorAsync()
    {
        try
        {
            if (!VerifyPackage())
                throw new FileNotFoundException("本地编辑器资源缺失，请重新解压整个应用包。");

            // Avoid generic runtime failure on clean Windows 10/LTSC machines.
            var runtime = CoreWebView2Environment.GetAvailableBrowserVersionString();
            if (string.IsNullOrWhiteSpace(runtime))
                throw new InvalidOperationException("缺少 Microsoft Edge WebView2 Runtime。");

            sessionFolder = Path.Combine(Path.GetTempPath(),
                "ShareReady-" + Guid.NewGuid().ToString("N"));
            var environment = await CoreWebView2Environment.CreateAsync(
                browserExecutableFolder: null, userDataFolder: sessionFolder);
            await editor.EnsureCoreWebView2Async(environment);
            var core = editor.CoreWebView2;
            core.Settings.AreDevToolsEnabled = false;
            core.Settings.AreDefaultContextMenusEnabled = false;
            core.Settings.IsStatusBarEnabled = false;
            core.Settings.IsPasswordAutosaveEnabled = false;
            core.Settings.IsGeneralAutofillEnabled = false;

            // Browser-native file picker, clipboard and drag/drop are enabled.
            // Local secure origin is mapped to the packaged files; never served
            // over a localhost port or third-party host.
            core.SetVirtualHostNameToFolderMapping(
                AppHost, WebRoot, CoreWebView2HostResourceAccessKind.DenyCors);

            core.NavigationStarting += (_, e) =>
            {
                if (!string.Equals(e.Uri, AppUrl, StringComparison.OrdinalIgnoreCase))
                    e.Cancel = true;
            };
            core.NewWindowRequested += (_, e) => e.Handled = true;
            core.PermissionRequested += (_, e) =>
            {
                // Clipboard paste with a user gesture works without granting
                // arbitrary unattended read of the system clipboard.
                if (!e.Uri.StartsWith("https://" + AppHost + "/", StringComparison.OrdinalIgnoreCase)
                    || e.PermissionKind == CoreWebView2PermissionKind.ClipboardRead
                    && !e.IsUserInitiated)
                {
                    e.State = CoreWebView2PermissionState.Deny;
                }
                else if (e.PermissionKind != CoreWebView2PermissionKind.ClipboardRead)
                {
                    e.State = CoreWebView2PermissionState.Deny;
                }
                e.SavesInProfile = false;
            };

            core.AddWebResourceRequestedFilter("*", CoreWebView2WebResourceContext.All);
            core.WebResourceRequested += (_, e) =>
            {
                if (!Uri.TryCreate(e.Request.Uri, UriKind.Absolute, out var uri))
                    return;
                if ((uri.Scheme == "http" || uri.Scheme == "https")
                    && !(uri.Scheme == "https"
                        && uri.Host.Equals(AppHost, StringComparison.OrdinalIgnoreCase)))
                {
                    e.Response = core.Environment.CreateWebResourceResponse(
                        null, 403, "Blocked by ShareReady",
                        "Content-Type: text/plain; charset=utf-8");
                }
            };
            core.DownloadStarting += OnDownloadStarting;
            core.NavigationCompleted += async (_, e) =>
            {
                if (!e.IsSuccess || webReady) return;
                webReady = true;
                if (smokeMode)
                    await RunNativeSmokeAsync();
            };
            core.Navigate(AppUrl);
        }
        catch (Exception ex)
        {
            if (smokeMode)
            {
                Console.Error.WriteLine("NATIVE_RUNTIME_INIT_FAILED: " + ex);
                Environment.ExitCode = 11;
            }
            else
            {
                MessageBox.Show("ShareReady 无法启动。\n\n"
                    + "请确认已完整解压程序文件，并安装 Microsoft Edge WebView2 Runtime。"
                    + "\n\n错误信息：" + ex.Message,
                    "ShareReady 启动失败", MessageBoxButtons.OK, MessageBoxIcon.Error);
            }
            Close();
        }
    }

    private void OnDownloadStarting(object? sender, CoreWebView2DownloadStartingEventArgs e)
    {
        // A download is never silently written to the default Downloads folder.
        // Only a current local-blob PNG can enter the native Save File workflow.
        e.Handled = true;
        var uri = e.DownloadOperation.Uri;
        if (smokeMode) Console.WriteLine("NATIVE_DOWNLOAD_START requested_cancel="
            + smokeCancelNextDownload + " uri=" + uri);
        if (!uri.StartsWith("blob:https://" + AppHost + "/", StringComparison.OrdinalIgnoreCase))
        {
            e.Cancel = true;
            NotifyDownload("blocked", uri);
            return;
        }

        if (smokeMode && smokeCancelNextDownload)
        {
            smokeCancelNextDownload = false;
            e.Cancel = true;
            NotifyDownload("canceled", uri);
            return;
        }

        string filename = Path.GetFileName(e.ResultFilePath);
        if (!filename.EndsWith(".png", StringComparison.OrdinalIgnoreCase))
            filename = "ShareReady.png";
        string? destination;
        if (smokeMode)
        {
            destination = Path.Combine(Path.GetTempPath(),
                "ShareReady-native-test-" + Guid.NewGuid().ToString("N") + ".png");
            smokeOutput = destination;
        }
        else
        {
            try
            {
                using var dialog = new SaveFileDialog
                {
                    Title = "保存隐私遮挡后的 PNG 图片",
                    Filter = "PNG 图片 (*.png)|*.png",
                    DefaultExt = "png",
                    AddExtension = true,
                    CheckPathExists = true,
                    OverwritePrompt = true,
                    FileName = filename
                };
                destination = dialog.ShowDialog(this) == DialogResult.OK
                    ? dialog.FileName : null;
            }
            catch (Exception error)
            {
                Debug.WriteLine("ShareReady save dialog failed: " + error.Message);
                e.Cancel = true;
                NotifyDownload("failed", uri);
                return;
            }
        }

        if (destination is null)
        {
            e.Cancel = true;
            NotifyDownload("canceled", uri);
            return;
        }
        e.ResultFilePath = destination;
        var operation = e.DownloadOperation;
        operation.StateChanged += (_, _) =>
        {
            if (operation.State == CoreWebView2DownloadState.Completed)
                NotifyDownload("saved", uri);
            else if (operation.State == CoreWebView2DownloadState.Interrupted)
                NotifyDownload("failed", uri);
        };
    }

    private void NotifyDownload(string state, string uri)
    {
        if (editor.IsDisposed || editor.CoreWebView2 is null) return;
        // Bind the native response to the exact one-use blob URL. Late
        // callbacks for a canceled/previous image cannot unlock a new export.
        if (smokeMode) Console.WriteLine("NATIVE_DOWNLOAD_NOTIFY state=" + state);
        var json = JsonSerializer.Serialize(new { type = "native-download", state, uri });
        editor.CoreWebView2.PostWebMessageAsJson(json);
    }

    private async Task<string> EvalAsync(string script) =>
        await editor.CoreWebView2.ExecuteScriptAsync(script);

    private async Task ClickAsRealMouseAsync(string selector)
    {
        // Download throttling deliberately distinguishes script-triggered
        // automatic downloads from user-initiated clicks. CDP mouse events
        // travel through Chromium's actual input routing and allow us to
        // test the real Windows button path, not HTMLElement.click().
        var script = "(() => { const r = document.querySelector(" +
            JsonSerializer.Serialize(selector) +
            ").getBoundingClientRect(); return {x:r.left+r.width/2,y:r.top+r.height/2}; })()";
        using var bounds = JsonDocument.Parse(await EvalAsync(script));
        var x = bounds.RootElement.GetProperty("x").GetDouble();
        var y = bounds.RootElement.GetProperty("y").GetDouble();
        var core = editor.CoreWebView2;
        await core.CallDevToolsProtocolMethodAsync("Input.dispatchMouseEvent",
            JsonSerializer.Serialize(new {
                type = "mouseMoved", x, y
            }));
        await core.CallDevToolsProtocolMethodAsync("Input.dispatchMouseEvent",
            JsonSerializer.Serialize(new {
                type = "mousePressed", x, y, button = "left", clickCount = 1
            }));
        await core.CallDevToolsProtocolMethodAsync("Input.dispatchMouseEvent",
            JsonSerializer.Serialize(new {
                type = "mouseReleased", x, y, button = "left", clickCount = 1
            }));
        if (smokeMode) Console.WriteLine("NATIVE_TRUSTED_MOUSE_CLICK " + selector);
    }

    private async Task<bool> WaitAsync(string script, int timeoutMs = 20000)
    {
        var clock = Stopwatch.StartNew();
        while (clock.ElapsedMilliseconds < timeoutMs)
        {
            var answer = await EvalAsync(script);
            if (answer == "true") return true;
            await Task.Delay(120);
        }
        return false;
    }

    private async Task RunNativeSmokeAsync()
    {
        // Native Windows + actual WebView2 integration. Fixtures contain
        // synthetic pixels only. This does not automate the OS file dialog,
        // physical OS drag operation, or every user's DPI setting.
        try
        {
            if (!await WaitAsync("Boolean(window.e2Debug?.ready)"))
                throw new Exception("WASM did not initialize in Windows WebView2");
            if (!editor.AllowExternalDrop)
                throw new Exception("Windows external file drop is disabled");
            if (!await WaitAsync("Boolean(document.querySelector('#pick')?.accept.includes('png'))"))
                throw new Exception("Native file-selection input not available");
            // Three browser intake paths in an actual native WebView2 window.
            // A synthetic generated PNG is used in every case.
            const string makePng = @"async (width,height) => {
                const canvas=document.createElement('canvas');
                canvas.width=width;canvas.height=height;
                const ctx=canvas.getContext('2d');
                ctx.fillStyle='#dae5ed';ctx.fillRect(0,0,width,height);
                ctx.fillStyle='#ff3300';ctx.fillRect(4,4,16,10);
                const blob=await new Promise(resolve=>canvas.toBlob(resolve,'image/png'));
                return new File([blob],'CI_SYNTHETIC.png',{type:'image/png'});
            }";
            await EvalAsync(@"(async()=>{
                const file=await (" + makePng + @")(80,60);
                const data=new DataTransfer();data.items.add(file);
                const pick=document.getElementById('pick');pick.files=data.files;
                pick.dispatchEvent(new Event('change',{bubbles:true}));
            })()");
            if (!await WaitAsync("window.e2Debug?.sourceSize[0]===80 && window.e2Debug.stage==='edit'"))
                throw new Exception("Windows file input failed");

            await EvalAsync(@"(async()=>{
                const file=await (" + makePng + @")(74,54);
                const data=new DataTransfer();data.items.add(file);
                document.dispatchEvent(new DragEvent('drop',{dataTransfer:data,bubbles:true,cancelable:true}));
            })()");
            if (!await WaitAsync("window.e2Debug?.sourceSize[0]===74"))
                throw new Exception("Windows drop intake failed");

            await EvalAsync(@"(async()=>{
                const file=await (" + makePng + @")(76,56);
                const data=new DataTransfer();data.items.add(file);
                document.body.dispatchEvent(new ClipboardEvent('paste',{
                    clipboardData:data,bubbles:true,cancelable:true
                }));
            })()");
            if (!await WaitAsync("window.e2Debug?.sourceSize[0]===76"))
                throw new Exception("Windows paste intake failed");

            await EvalAsync(@"document.getElementById('createMask').click()");
            if (!await WaitAsync("window.e2Debug?.regions.length===1"))
                throw new Exception("Windows editor unable to create mask");
            // The same compiled desktop UI must expose the 8-point geometry
            // controls, not an older one-corner demo asset.
            if (!await WaitAsync("document.querySelectorAll('.mask.selected .handle[data-dir]').length===8"))
                throw new Exception("Windows editor missing V1.3-02 eight-direction handles");
            await EvalAsync("document.getElementById('zoomIn').click()");
            if (!await WaitAsync("window.e2Debug?.zoom > 1"))
                throw new Exception("Native image zoom does not update");
            await EvalAsync("document.getElementById('zoomFit').click()");
            await EvalAsync(@"(()=>{
                for(const [key,value] of [['precX','6'],['precY','8'],['precW','20'],['precH','11']]){
                    const field=document.getElementById(key);
                    field.value=value;
                    field.dispatchEvent(new Event('change',{bubbles:true}));
                }
                document.getElementById('check').click();
            })()");
            if (!await WaitAsync("window.e2Debug?.stage==='review' && window.e2Debug.outputBytes>0"))
                throw new Exception("Windows WebView2 Rust PNG review failed");
            // Exercise cancel first, then immediately retry the very same
            // verified PNG. The test invokes WebView2's actual DownloadStarting
            // cancellation branch rather than faking a host status message.
            smokeCancelNextDownload = true;
            await ClickAsRealMouseAsync("#download");
            if (!await WaitAsync("window.__shareReadyNativeDownload==='canceled'"))
                throw new Exception("Native save cancellation was not reported; state="
                    + await EvalAsync("window.e2Debug?.nativeDownloadState"));
            if (!await WaitAsync("!document.getElementById('download').disabled"))
                throw new Exception("Save cancel did not restore retry button");
            if (await EvalAsync("window.e2Debug?.stage") != "\"review\"")
                throw new Exception("Native canceled save destroyed review");
            await ClickAsRealMouseAsync("#download");
            if (!await WaitAsync("Boolean(window.__shareReadyNativeDownload==='saved')", 20000)
                || smokeOutput is null
                || !File.Exists(smokeOutput))
                throw new Exception("Windows native PNG Save dialog/download flow failed");

            using (var image = new Bitmap(smokeOutput))
            {
                if (image.Width != 76 || image.Height != 56)
                    throw new Exception("Windows output geometry mismatch");
                if (image.GetPixel(10, 12).ToArgb() != Color.FromArgb(255, 17, 24, 39).ToArgb())
                    throw new Exception("Native masked pixel did not contain Rust cover");
                if (image.GetPixel(0, 0).ToArgb() == Color.FromArgb(255, 17, 24, 39).ToArgb())
                    throw new Exception("Unmasked source pixel was lost");
            }
            await EvalAsync(@"document.getElementById('copy').click()");
            if (!await WaitAsync("window.e2Debug?.copyState!=='not_attempted'"))
                throw new Exception("Native clipboard operation neither succeeded nor returned a truthful failure");
            var size = await EvalAsync("JSON.stringify({width:innerWidth,height:innerHeight,dpr:devicePixelRatio})");
            if (string.IsNullOrWhiteSpace(size) || size == "null")
                throw new Exception("WebView2 DPI/viewport could not be measured");
            Console.WriteLine("SHAREREADY_V13_NATIVE_WEBVIEW2_WASM_FILE_DROP_PASTE_MASK_SAVE_CLIPBOARD_PASS");
            Console.WriteLine("NATIVE_VIEWPORT_DPI=" + size);
            Environment.ExitCode = 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine("NATIVE_E2E_FAILED: " + ex);
            Environment.ExitCode = 12;
        }
        finally
        {
            if (smokeOutput != null)
            {
                try { File.Delete(smokeOutput); } catch (IOException) { }
            }
            Close();
        }
    }

    private void CleanupSession()
    {
        try { editor.Dispose(); } catch { }
        if (sessionFolder is null) return;
        try { Directory.Delete(sessionFolder, recursive: true); }
        catch (IOException) { }
        catch (UnauthorizedAccessException) { }
    }
}
