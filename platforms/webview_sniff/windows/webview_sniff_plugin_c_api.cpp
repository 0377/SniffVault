#include "include/webview_sniff/webview_sniff_plugin_c_api.h"

#include <flutter/plugin_registrar_windows.h>

#include "webview_sniff_plugin.h"

void WebviewSniffPluginCApiRegisterWithRegistrar(
    FlutterDesktopPluginRegistrarRef registrar) {
  webview_sniff::WebviewSniffPlugin::RegisterWithRegistrar(
      flutter::PluginRegistrarManager::GetInstance()
          ->GetRegistrar<flutter::PluginRegistrarWindows>(registrar));
}
