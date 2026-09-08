# platforms/

- `webview_sniff/`：插件职责是 Cookie 仓（`cookieHeaderFor` / `clearCookies`）与 Android `isTelevision` Channel。不是 WebView 嗅探实现。
- iOS Share Extension **尚未实现**。
- 浏览嗅探在 Flutter 侧用官方 `webview_flutter` 的 NavigationDelegate + 注入脚本（Android / iOS / macOS）。官方 `webview_flutter` 没有 Windows 实现。
