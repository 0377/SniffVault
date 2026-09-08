#include "webview_sniff_plugin.h"

#include <windows.h>

#include <flutter/method_channel.h>
#include <flutter/plugin_registrar_windows.h>
#include <flutter/standard_method_codec.h>
#include <wrl.h>

#include <memory>
#include <string>

namespace webview_sniff {

namespace {

using Microsoft::WRL::Callback;
using Microsoft::WRL::ComPtr;

std::string Utf8FromUtf16(const wchar_t *utf16) {
  if (utf16 == nullptr || utf16[0] == 0) {
    return {};
  }
  const int length = WideCharToMultiByte(CP_UTF8, 0, utf16, -1, nullptr, 0,
                                         nullptr, nullptr);
  if (length <= 1) {
    return {};
  }
  std::string utf8(static_cast<size_t>(length - 1), '\0');
  WideCharToMultiByte(CP_UTF8, 0, utf16, -1, utf8.data(), length, nullptr,
                      nullptr);
  return utf8;
}

std::wstring Utf16FromUtf8(const std::string &utf8) {
  if (utf8.empty()) {
    return {};
  }
  const int length =
      MultiByteToWideChar(CP_UTF8, 0, utf8.c_str(), -1, nullptr, 0);
  if (length <= 1) {
    return {};
  }
  std::wstring utf16(static_cast<size_t>(length - 1), L'\0');
  MultiByteToWideChar(CP_UTF8, 0, utf8.c_str(), -1, utf16.data(), length);
  return utf16;
}

HWND CreateHiddenHostWindow() {
  return CreateWindowExW(0, L"STATIC", L"webview_sniff_cookies", WS_POPUP, 0, 0,
                         1, 1, nullptr, nullptr, GetModuleHandleW(nullptr),
                         nullptr);
}

}  // namespace

// static
void WebviewSniffPlugin::RegisterWithRegistrar(
    flutter::PluginRegistrarWindows *registrar) {
  HWND hwnd = nullptr;
  if (registrar->GetView() != nullptr) {
    hwnd = registrar->GetView()->GetNativeWindow();
  }
  auto plugin = std::make_unique<WebviewSniffPlugin>(hwnd);

  auto channel =
      std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
          registrar->messenger(), "webview_sniff/cookies",
          &flutter::StandardMethodCodec::GetInstance());

  channel->SetMethodCallHandler(
      [plugin_pointer = plugin.get()](const auto &call, auto result) {
        plugin_pointer->HandleMethodCall(call, std::move(result));
      });

  registrar->AddPlugin(std::move(plugin));
}

WebviewSniffPlugin::WebviewSniffPlugin(HWND parent_hwnd) {
  (void)parent_hwnd;
  hwnd_ = CreateHiddenHostWindow();
  owns_hwnd_ = hwnd_ != nullptr;
}

WebviewSniffPlugin::~WebviewSniffPlugin() {
  if (controller_) {
    controller_->Close();
  }
  if (owns_hwnd_ && hwnd_ != nullptr) {
    DestroyWindow(hwnd_);
  }
}

void WebviewSniffPlugin::HandleMethodCall(
    const flutter::MethodCall<flutter::EncodableValue> &method_call,
    std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
  if (method_call.method_name() == "cookieHeaderFor") {
    const auto *url = std::get_if<std::string>(method_call.arguments());
    if (url == nullptr || url->empty()) {
      result->Success();
      return;
    }
    CookieHeaderFor(*url, std::move(result));
    return;
  }
  if (method_call.method_name() == "clearCookies") {
    ClearCookies(std::move(result));
    return;
  }
  if (method_call.method_name() == "setUserDataFolder") {
    const auto *folder = std::get_if<std::string>(method_call.arguments());
    if (folder != nullptr) {
      SetUserDataFolder(*folder);
    }
    result->Success();
    return;
  }
  result->NotImplemented();
}

void WebviewSniffPlugin::SetUserDataFolder(const std::string &folder) {
  if (folder.empty()) {
    return;
  }
  const std::wstring next = Utf16FromUtf8(folder);
  if (user_data_folder_ == next && init_state_ != InitState::kFailed) {
    return;
  }
  ResetEnvironment();
  user_data_folder_ = std::move(next);
}

void WebviewSniffPlugin::ResetEnvironment() {
  if (controller_) {
    controller_->Close();
    controller_.Reset();
  }
  cookie_manager_.Reset();
  init_state_ = InitState::kIdle;
  pending_.clear();
}

void WebviewSniffPlugin::EnsureCookieManager(
    std::function<void(ICoreWebView2CookieManager *)> done) {
  if (init_state_ == InitState::kReady) {
    done(cookie_manager_.Get());
    return;
  }
  if (init_state_ == InitState::kFailed || hwnd_ == nullptr) {
    done(nullptr);
    return;
  }
  pending_.push_back(std::move(done));
  if (init_state_ == InitState::kPending) {
    return;
  }
  init_state_ = InitState::kPending;
  const wchar_t *udf =
      user_data_folder_.empty() ? nullptr : user_data_folder_.c_str();
  const HRESULT hr = CreateCoreWebView2EnvironmentWithOptions(
      nullptr, udf, nullptr,
      Callback<ICoreWebView2CreateCoreWebView2EnvironmentCompletedHandler>(
          [this](HRESULT error_code, ICoreWebView2Environment *env) -> HRESULT {
            auto finish = [this](ICoreWebView2CookieManager *manager) {
              init_state_ =
                  manager == nullptr ? InitState::kFailed : InitState::kReady;
              cookie_manager_ = manager;
              auto waiting = std::move(pending_);
              pending_.clear();
              for (auto &callback : waiting) {
                callback(manager);
              }
            };
            if (FAILED(error_code) || env == nullptr) {
              finish(nullptr);
              return S_OK;
            }
            env->CreateCoreWebView2Controller(
                hwnd_,
                Callback<
                    ICoreWebView2CreateCoreWebView2ControllerCompletedHandler>(
                    [this, finish](HRESULT controller_error,
                                   ICoreWebView2Controller *controller)
                        -> HRESULT {
                      if (FAILED(controller_error) || controller == nullptr) {
                        finish(nullptr);
                        return S_OK;
                      }
                      controller_ = controller;
                      ComPtr<ICoreWebView2> webview;
                      if (FAILED(controller->get_CoreWebView2(&webview)) ||
                          !webview) {
                        finish(nullptr);
                        return S_OK;
                      }
                      ComPtr<ICoreWebView2_2> webview2;
                      if (FAILED(webview.As(&webview2)) || !webview2) {
                        finish(nullptr);
                        return S_OK;
                      }
                      ComPtr<ICoreWebView2CookieManager> manager;
                      if (FAILED(webview2->get_CookieManager(&manager)) ||
                          !manager) {
                        finish(nullptr);
                        return S_OK;
                      }
                      finish(manager.Get());
                      return S_OK;
                    })
                    .Get());
            return S_OK;
          })
          .Get());
  if (FAILED(hr)) {
    init_state_ = InitState::kFailed;
    auto waiting = std::move(pending_);
    pending_.clear();
    for (auto &callback : waiting) {
      callback(nullptr);
    }
  }
}

void WebviewSniffPlugin::CookieHeaderFor(
    const std::string &url,
    std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
  auto *raw_result = result.release();
  EnsureCookieManager([raw_result, url](ICoreWebView2CookieManager *manager) {
    if (manager == nullptr) {
      std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> reply(
          raw_result);
      reply->Success();
      return;
    }
    const std::wstring uri = Utf16FromUtf8(url);
    const HRESULT hr = manager->GetCookies(
        uri.c_str(),
        Callback<ICoreWebView2GetCookiesCompletedHandler>(
            [raw_result](HRESULT error_code,
                         ICoreWebView2CookieList *list) -> HRESULT {
              std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>>
                  reply(raw_result);
              if (FAILED(error_code) || list == nullptr) {
                reply->Success();
                return S_OK;
              }
              UINT count = 0;
              list->get_Count(&count);
              if (count == 0) {
                reply->Success();
                return S_OK;
              }
              std::string header;
              for (UINT i = 0; i < count; ++i) {
                ComPtr<ICoreWebView2Cookie> cookie;
                if (FAILED(list->GetValueAtIndex(i, &cookie)) || !cookie) {
                  continue;
                }
                LPWSTR name = nullptr;
                LPWSTR value = nullptr;
                cookie->get_Name(&name);
                cookie->get_Value(&value);
                if (name != nullptr && value != nullptr) {
                  if (!header.empty()) {
                    header += "; ";
                  }
                  header += Utf8FromUtf16(name);
                  header += "=";
                  header += Utf8FromUtf16(value);
                }
                CoTaskMemFree(name);
                CoTaskMemFree(value);
              }
              if (header.empty()) {
                reply->Success();
              } else {
                reply->Success(flutter::EncodableValue(header));
              }
              return S_OK;
            })
            .Get());
    if (FAILED(hr)) {
      std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> reply(
          raw_result);
      reply->Success();
    }
  });
}

void WebviewSniffPlugin::ClearCookies(
    std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
  auto *raw_result = result.release();
  EnsureCookieManager([raw_result](ICoreWebView2CookieManager *manager) {
    std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> reply(
        raw_result);
    if (manager != nullptr) {
      manager->DeleteAllCookies();
    }
    reply->Success();
  });
}

}  // namespace webview_sniff
