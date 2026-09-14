package com.videosniffing.video_sniffing

import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.util.Log
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
        val abi = Build.SUPPORTED_ABIS.firstOrNull()
        if (abi == null) {
            Log.e(TAG, "ensureFfmpegInstalled: no supported ABI")
            return false
        }

        val assetPath = "ffmpeg/$abi/ffmpeg"
        val destDir = File(dataDir, "bin")
        if (!destDir.exists() && !destDir.mkdirs()) {
            Log.e(TAG, "ensureFfmpegInstalled: failed to create ${destDir.absolutePath}")
            return false
        }

        val dest = File(destDir, "ffmpeg")
        val versionFile = File(destDir, "ffmpeg.version")
        val bundledVersion = readBundledFfmpegVersion()
        if (bundledVersion == null) {
            Log.e(TAG, "ensureFfmpegInstalled: missing assets/ffmpeg/version.txt")
            return false
        }

        if (
            dest.isFile &&
            dest.length() > 0L &&
            versionFile.isFile &&
            versionFile.readText().trim() == bundledVersion
        ) {
            return true
        }

        return try {
            assets.open(assetPath).use { input ->
                dest.outputStream().use { output ->
                    input.copyTo(output)
                }
            }
            if (!dest.setExecutable(true, false) || !dest.isFile) {
                Log.e(TAG, "ensureFfmpegInstalled: chmod failed for ${dest.absolutePath}")
                dest.delete()
                return false
            }
            versionFile.writeText(bundledVersion)
            true
        } catch (e: Exception) {
            Log.e(TAG, "ensureFfmpegInstalled failed for abi=$abi asset=$assetPath", e)
            dest.delete()
            versionFile.delete()
            false
        }
    }

    private fun readBundledFfmpegVersion(): String? {
        return try {
            assets.open("ffmpeg/version.txt").use { input ->
                input.reader().readText().trim().takeIf { it.isNotEmpty() }
            }
        } catch (e: Exception) {
            Log.e(TAG, "readBundledFfmpegVersion failed", e)
            null
        }
    }

    companion object {
        private const val TAG = "VideoSniffing"
        private const val DEVICE_CHANNEL = "com.videosniffing.video_sniffing/device"
    }
}
