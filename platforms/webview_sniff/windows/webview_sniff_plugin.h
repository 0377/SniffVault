#ifndef FLUTTER_PLUGIN_WEBVIEW_SNIFF_PLUGIN_H_
#define FLUTTER_PLUGIN_WEBVIEW_SNIFF_PLUGIN_H_

#include <flutter/method_channel.h>
#include <flutter/plugin_registrar_windows.h>
#include <wrl/client.h>
#include <WebView2.h>

#include <functional>
#include <memory>
#include <string>
#include <vector>

namespace webview_sniff {

class WebviewSniffPlugin : public flutter::Plugin {
 public:
  static void RegisterWithRegistrar(flutter::PluginRegistrarWindows *registrar);

  explicit WebviewSniffPlugin(HWND parent_hwnd);

  virtual ~WebviewSniffPlugin();

  WebviewSniffPlugin(const WebviewSniffPlugin &) = delete;
  WebviewSniffPlugin &operator=(const WebviewSniffPlugin &) = delete;

  void HandleMethodCall(
      const flutter::MethodCall<flutter::EncodableValue> &method_call,
      std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result);

 private:
  void EnsureCookieManager(
      std::function<void(ICoreWebView2CookieManager *)> done);
  void CookieHeaderFor(
      const std::string &url,
      std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result);
  void ClearCookies(
      std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result);
  void SetUserDataFolder(const std::string &folder);
  void ProbeWebView2(
      std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result);
  void ResetEnvironment();

  HWND hwnd_ = nullptr;
  bool owns_hwnd_ = false;
  Microsoft::WRL::ComPtr<ICoreWebView2Controller> controller_;
  Microsoft::WRL::ComPtr<ICoreWebView2CookieManager> cookie_manager_;
  enum class InitState { kIdle, kPending, kReady, kFailed };
  InitState init_state_ = InitState::kIdle;
  std::vector<std::function<void(ICoreWebView2CookieManager *)>> pending_;
  std::wstring user_data_folder_;
};

}  // namespace webview_sniff

#endif  // FLUTTER_PLUGIN_WEBVIEW_SNIFF_PLUGIN_H_
