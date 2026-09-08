package com.videosniffing.video_sniffing

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import io.flutter.embedding.android.FlutterActivity

class MainActivity : FlutterActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        rewriteIngressIntent(intent)
        super.onCreate(savedInstanceState)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        rewriteIngressIntent(intent)
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
}
