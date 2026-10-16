using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;

namespace ShareReady.Desktop;

internal static class Program
{
    [STAThread]
    private static void Main()
    {
        ApplicationConfiguration.Initialize();
        Application.Run(new ShareReadyWindow());
    }
}

/// <summary>
/// Native Windows frame hosting the locally bundled Rust/WASM editor.
/// No local HTTP listener, cloud endpoint, or browser tab is required.
/// </summary>
internal sealed class ShareReadyWindow : Form
{
    private const string AppUrl = "https://shareready.local/index.html";
    private const string AppHost = "shareready.local";
    private readonly WebView2 editor = new() { Dock = DockStyle.Fill };
    private string? sessionFolder;

    public ShareReadyWindow()
    {
        Text = "ShareReady — 分享前隐私遮挡";
        MinimumSize = new Size(940, 650);
        Size = new Size(1360, 870);
        StartPosition = FormStartPosition.CenterScreen;
        BackColor = Color.FromArgb(248, 250, 252);
        Controls.Add(editor);
        Shown += async (_, _) => await InitializeEditorAsync();
        FormClosed += (_, _) => CleanupSession();
    }

    private async Task InitializeEditorAsync()
    {
        try
        {
            var webRoot = Path.Combine(AppContext.BaseDirectory, "www");
            foreach (var file in new[]
            {
                "index.html", "app.js", "pkg/shareready_e1.js",
                "pkg/shareready_e1_bg.wasm"
            })
            {
                var path = Path.Combine(webRoot, file.Replace('/', Path.DirectorySeparatorChar));
                if (!File.Exists(path))
                    throw new FileNotFoundException("缺少程序资源，请重新解压完整安装包。", file);
            }

            // WebView2 profile data is isolated per launch, never shared with
            // a regular Edge/Chrome browsing profile.
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

            // A virtual HTTPS origin makes WebAssembly & clipboard APIs usable
            // without a Python/PowerShell HTTP server or outside connectivity.
            core.SetVirtualHostNameToFolderMapping(AppHost, webRoot,
                CoreWebView2HostResourceAccessKind.DenyCors);

            core.NavigationStarting += (_, args) =>
            {
                if (!string.Equals(args.Uri, AppUrl, StringComparison.OrdinalIgnoreCase))
                    args.Cancel = true;
            };
            core.NewWindowRequested += (_, args) => args.Handled = true;

            // Block outgoing HTTP(S) subrequests before they reach the network.
            // Microsoft documents that WebResourceRequested does not fire for
            // files served by SetVirtualHostNameToFolderMapping.
            core.AddWebResourceRequestedFilter("*", CoreWebView2WebResourceContext.All);
            core.WebResourceRequested += (_, args) =>
            {
                if (!Uri.TryCreate(args.Request.Uri, UriKind.Absolute, out var uri))
                    return;
                if ((uri.Scheme == "http" || uri.Scheme == "https")
                    && !(uri.Scheme == "https" && uri.Host == AppHost))
                {
                    args.Response = core.Environment.CreateWebResourceResponse(
                        null, 403, "Blocked by ShareReady", "Content-Type: text/plain");
                }
            };

            core.Navigate(AppUrl);
        }
        catch (Exception error)
        {
            MessageBox.Show(
                "ShareReady 无法启动内置编辑器。请确保系统已安装 Microsoft Edge WebView2 Runtime，"
                + "并且完整解压应用文件。\n\n" + error.Message,
                "ShareReady 启动失败", MessageBoxButtons.OK, MessageBoxIcon.Error);
            Close();
        }
    }

    private void CleanupSession()
    {
        try { editor.Dispose(); } catch { /* WebView2 may already be disposed */ }
        // Browser child processes might briefly hold files after the window
        // closes. Do not claim a guaranteed secure erase of cached pixels.
        if (sessionFolder is null) return;
        try { Directory.Delete(sessionFolder, recursive: true); }
        catch (IOException) { }
        catch (UnauthorizedAccessException) { }
    }
}
