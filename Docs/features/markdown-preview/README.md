# Markdown preview

[All features](../../README.md) · [Main README](../../../README.md)

Pitex opens `.md` and `.markdown` files in the text editor.
The inspector shows a Markdown preview in place of the PDF column.
The preview includes KaTeX math with `$…$`, `$$…$$`, `\[…\]`, and `\(…\)`.

Pitex is also an **Open With** application for Markdown files on all platforms.
To make it the default, select **Settings → Markdown → Use for .md Files**.
The **Use for .tex Files** setting supplies the same function for TeX files.

| Setting | Function |
|---|---|
| **Live preview** | Updates the preview during text entry. If you disable it, the preview updates on save. |
| **Sync scrolling** | Keeps the editor and preview at the same source line during a scroll. |
| **Preview theme** | Uses **Match app**, **Light**, or **Dark**. **Match app** uses the application appearance. |
| Font size | Sets the preview font size. |

These settings are in **Settings → Markdown**, after the **TeX Compile** tab.

The preview shows raw HTML.
Its content-security policy blocks scripts, event handlers, and `javascript:` links.
HTTP, HTTPS, and `mailto` links open in the browser.
Relative file links open in the default application.
Pitex blocks executable files and other URL schemes.
It ignores links to missing files.

On Linux, this preview needs WebKitGTK 6.0 (`libwebkitgtk-6.0-4`).
The Ubuntu 24.04 and Ubuntu 22.04 packages include Markdown preview with this dependency.
Windows uses the Microsoft Edge WebView2 Evergreen runtime for Markdown and equation previews.
The installer and portable ZIP include `WebView2Loader.dll`.
If the runtime is missing, install [WebView2 Evergreen](https://developer.microsoft.com/microsoft-edge/webview2/).
