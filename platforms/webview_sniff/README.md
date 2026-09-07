# webview_sniff

MethodChannel `webview_sniff/device` (`isTelevision`, Android) and
`webview_sniff/cookies` (`cookieHeaderFor`, `clearCookies`) on
Android / iOS / macOS / Windows.

Cookie export reads the platform WebView cookie store:

- Android: `CookieManager.getCookie`
- iOS / macOS: `WKHTTPCookieStore` joined as `name=value; ...`
- Windows: WebView2 `ICoreWebView2CookieManager`

If a platform cannot read cookies, `cookieHeaderFor` returns null.
Dart callers treat missing plugins and platform errors as null; they must
not throw.
