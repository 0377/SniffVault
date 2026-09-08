import Cocoa
import FlutterMacOS
import WebKit

public class WebviewSniffPlugin: NSObject, FlutterPlugin {
  public static func register(with registrar: FlutterPluginRegistrar) {
    let channel = FlutterMethodChannel(
      name: "webview_sniff/cookies",
      binaryMessenger: registrar.messenger
    )
    let instance = WebviewSniffPlugin()
    registrar.addMethodCallDelegate(instance, channel: channel)
  }

  public func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    switch call.method {
    case "cookieHeaderFor":
      cookieHeaderFor(call.arguments as? String, result: result)
    case "clearCookies":
      clearCookies(result: result)
    default:
      result(FlutterMethodNotImplemented)
    }
  }
}

private func cookieHeaderFor(_ urlString: String?, result: @escaping FlutterResult) {
  guard let urlString, let url = URL(string: urlString) else {
    result(nil)
    return
  }
  WKWebsiteDataStore.default().httpCookieStore.getAllCookies { cookies in
    let header = cookieHeader(from: cookies, for: url)
    DispatchQueue.main.async {
      result(header)
    }
  }
}

private func clearCookies(result: @escaping FlutterResult) {
  WKWebsiteDataStore.default().removeData(
    ofTypes: [WKWebsiteDataTypeCookies],
    modifiedSince: Date.distantPast
  ) {
    DispatchQueue.main.async {
      result(nil)
    }
  }
}

func cookieHeader(from cookies: [HTTPCookie], for url: URL) -> String? {
  let matched = cookies.filter { cookieMatches($0, url: url) }
  if matched.isEmpty {
    return nil
  }
  return matched.map { "\($0.name)=\($0.value)" }.joined(separator: "; ")
}

func cookieMatches(_ cookie: HTTPCookie, url: URL) -> Bool {
  if let expires = cookie.expiresDate, expires < Date() {
    return false
  }
  if cookie.isSecure && url.scheme?.lowercased() != "https" {
    return false
  }
  guard let host = url.host?.lowercased() else {
    return false
  }
  if !domainMatches(cookie.domain, host: host) {
    return false
  }
  let path = url.path.isEmpty ? "/" : url.path
  return pathMatches(cookie.path, urlPath: path)
}

func domainMatches(_ cookieDomain: String, host: String) -> Bool {
  let domain = cookieDomain.lowercased()
  let domainNoDot = domain.hasPrefix(".") ? String(domain.dropFirst()) : domain
  if host == domainNoDot {
    return true
  }
  return host.hasSuffix("." + domainNoDot)
}

func pathMatches(_ cookiePath: String, urlPath: String) -> Bool {
  if cookiePath.isEmpty || cookiePath == "/" {
    return true
  }
  if urlPath == cookiePath {
    return true
  }
  if cookiePath.hasSuffix("/") {
    return urlPath.hasPrefix(cookiePath)
  }
  return urlPath.hasPrefix(cookiePath + "/")
}
