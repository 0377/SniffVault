# platforms/

- `webview_sniff/`：插件职责是 Cookie 仓（`cookieHeaderFor` / `clearCookies`）与 Android `isTelevision` Channel。不是 WebView 嗅探实现。
- `share_ingress/`：托管 iOS Share Extension 源文件（`ios/ShareExtension/`），Xcode target 引用；Android 分享由 `app/android/.../MainActivity.kt` 处理，不经此插件。一期无 Dart API（`lib/share_ingress.dart` 为空 library）。
- 浏览嗅探在 Flutter 侧用官方 `webview_flutter`（Android / iOS / macOS）。Windows 通过 `webview_win_floating` 实现 WebView2；Cookie 与 `webview_sniff` 共用 `{ApplicationSupport}/webview_profile`。
