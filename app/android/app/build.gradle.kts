plugins {
    id("com.android.application")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

android {
    namespace = "com.videosniffing.video_sniffing"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        // TODO: Specify your own unique Application ID (https://developer.android.com/studio/build/application-id.html).
        applicationId = "com.videosniffing.video_sniffing"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName
    }

    buildTypes {
        release {
            // TODO: Add your own signing config for the release build.
            // Signing with the debug keys for now, so `flutter run --release` works.
            signingConfig = signingConfigs.getByName("debug")
        }
    }

}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

flutter {
    source = "../.."
}

val engineRoot = rootProject.projectDir.parentFile.parentFile.resolve("engine")
val vendorFfmpeg = engineRoot.resolve("vendor/ffmpeg")
val allAndroidAbis = listOf("arm64-v8a", "armeabi-v7a", "x86", "x86_64")

fun flutterPlatformToAbi(platform: String): String? {
    return when (platform) {
        "android-arm64" -> "arm64-v8a"
        "android-arm" -> "armeabi-v7a"
        "android-x64" -> "x86_64"
        "android-x86" -> "x86"
        else -> null
    }
}

fun resolveAbisToCopy(): List<String> {
    val platforms = project.findProperty("target-platform")?.toString()
        ?.split(",")
        ?.map { it.trim() }
        ?.filter { it.isNotEmpty() }
        ?: emptyList()
    if (platforms.isEmpty()) {
        return allAndroidAbis
    }
    val mapped = platforms.mapNotNull { flutterPlatformToAbi(it) }.distinct()
    return if (mapped.isEmpty()) allAndroidAbis else mapped
}

fun isReleaseBuildRequested(): Boolean {
    return gradle.startParameter.taskNames.any { task ->
        task.contains("release", ignoreCase = true)
    }
}

tasks.register("copyFfmpegAssets") {
    doLast {
        val ffmpegRoot = file("src/main/assets/ffmpeg")
        ffmpegRoot.mkdirs()

        val abisToCopy = resolveAbisToCopy()
        val missing = mutableListOf<String>()
        var copied = 0

        for (abi in abisToCopy) {
            val src = vendorFfmpeg.resolve("android-$abi/ffmpeg")
            if (!src.exists()) {
                missing.add(abi)
                continue
            }
            val dst = File(ffmpegRoot, "$abi/ffmpeg")
            dst.parentFile.mkdirs()
            src.copyTo(dst, overwrite = true)
            copied++
        }

        File(ffmpegRoot, "version.txt").writeText(flutter.versionCode.toString())

        if (missing.isNotEmpty()) {
            val message =
                "Missing ffmpeg for ABI(s): ${missing.joinToString()}. " +
                    "Run: (cd engine && ./scripts/fetch_ffmpeg_android.sh)"
            if (isReleaseBuildRequested()) {
                throw GradleException(message)
            }
            logger.warn(message)
        }

        if (isReleaseBuildRequested() && abisToCopy.isNotEmpty() && copied == 0) {
            throw GradleException(
                "No ffmpeg binaries copied for release build. " +
                    "Run: (cd engine && ./scripts/fetch_ffmpeg_android.sh)",
            )
        }
    }
}

tasks.named("preBuild") {
    dependsOn("copyFfmpegAssets")
}
