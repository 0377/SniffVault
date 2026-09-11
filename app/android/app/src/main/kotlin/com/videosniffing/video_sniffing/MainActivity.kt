package com.videosniffing.video_sniffing

import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import java.io.File

class MainActivity : FlutterActivity() {
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            DEVICE_CHANNEL,
        ).setMethodCallHandler { call, result ->
            when (call.method) {
                "ensureFfmpeg" -> {
                    val dataDir = call.argument<String>("dataDir")
                    if (dataDir.isNullOrBlank()) {
                        result.error("invalid_arg", "dataDir is required", null)
                        return@setMethodCallHandler
                    }
                    result.success(ensureFfmpegInstalled(dataDir))
                }
                else -> result.notImplemented()
            }
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        rewriteIngressIntent(intent)
        super.onCreate(savedInstanceState)
    }

    override fun onNewIntent(intent: Intent) {
        rewriteIngressIntent(intent)
        super.onNewIntent(getIntent())
    }

    private fun rewriteIngressIntent(source: Intent?) {
        val uri = toIngressUri(source) ?: return
        setIntent(Intent(Intent.ACTION_VIEW, uri))
    }

    private fun toIngressUri(intent: Intent?): Uri? {
        if (intent == null) return null
        when (intent.action) {
            Intent.ACTION_SEND -> {
                if (intent.type != "text/plain") return null
                val text = intent.getStringExtra(Intent.EXTRA_TEXT)?.trim()
                    ?: return null
                if (text.isEmpty()) return null
                return Uri.Builder()
                    .scheme("sniffvault")
                    .authority("add")
                    .appendQueryParameter("url", text)
                    .build()
            }
            Intent.ACTION_VIEW -> {
                val data = intent.data
                if (data?.scheme == "sniffvault" && data.host == "add") {
                    return data
                }
            }
        }
        return null
    }

    private fun ensureFfmpegInstalled(dataDir: String): Boolean {
        val abi = Build.SUPPORTED_ABIS.firstOrNull() ?: return false
        val assetPath = "ffmpeg/$abi/ffmpeg"
        val destDir = File(dataDir, "bin")
        if (!destDir.exists() && !destDir.mkdirs()) {
            return false
        }
        val dest = File(destDir, "ffmpeg")
        if (dest.isFile && dest.length() > 0L) {
            return true
        }
        return try {
            assets.open(assetPath).use { input ->
                dest.outputStream().use { output ->
                    input.copyTo(output)
                }
            }
            dest.setExecutable(true, false)
            dest.isFile
        } catch (_: Exception) {
            dest.delete()
            false
        }
    }

    companion object {
        private const val DEVICE_CHANNEL = "com.videosniffing.video_sniffing/device"
    }
}
