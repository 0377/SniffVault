package com.videosniffing.webview_sniff

import android.content.Context
import android.content.res.Configuration
import android.webkit.CookieManager
import io.flutter.embedding.engine.plugins.FlutterPlugin
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.flutter.plugin.common.MethodChannel.MethodCallHandler
import io.flutter.plugin.common.MethodChannel.Result

class WebviewSniffPlugin :
    FlutterPlugin,
    MethodCallHandler {
    private lateinit var deviceChannel: MethodChannel
    private lateinit var cookiesChannel: MethodChannel
    private var applicationContext: Context? = null

    override fun onAttachedToEngine(flutterPluginBinding: FlutterPlugin.FlutterPluginBinding) {
        applicationContext = flutterPluginBinding.applicationContext
        deviceChannel =
            MethodChannel(
                flutterPluginBinding.binaryMessenger,
                "webview_sniff/device",
            )
        deviceChannel.setMethodCallHandler(this)
        cookiesChannel =
            MethodChannel(
                flutterPluginBinding.binaryMessenger,
                "webview_sniff/cookies",
            )
        cookiesChannel.setMethodCallHandler(this)
    }

    override fun onMethodCall(
        call: MethodCall,
        result: Result,
    ) {
        when (call.method) {
            "isTelevision" -> result.success(isTelevision())
            "cookieHeaderFor" -> result.success(cookieHeaderFor(call.arguments as? String))
            "clearCookies" -> clearCookies(result)
            else -> result.notImplemented()
        }
    }

    private fun isTelevision(): Boolean {
        val context = applicationContext ?: return false
        val uiMode = context.resources.configuration.uiMode
        return uiMode and Configuration.UI_MODE_TYPE_MASK ==
            Configuration.UI_MODE_TYPE_TELEVISION
    }

    private fun cookieHeaderFor(url: String?): String? {
        if (url.isNullOrEmpty()) {
            return null
        }
        return try {
            val header = CookieManager.getInstance().getCookie(url)
            if (header.isNullOrEmpty()) null else header
        } catch (_: Exception) {
            null
        }
    }

    private fun clearCookies(result: Result) {
        try {
            val manager = CookieManager.getInstance()
            manager.removeAllCookies {
                manager.flush()
                result.success(null)
            }
        } catch (_: Exception) {
            result.success(null)
        }
    }

    override fun onDetachedFromEngine(binding: FlutterPlugin.FlutterPluginBinding) {
        deviceChannel.setMethodCallHandler(null)
        cookiesChannel.setMethodCallHandler(null)
        applicationContext = null
    }
}
