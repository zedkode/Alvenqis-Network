import java.util.zip.ZipFile

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
}

val alvenqisNativeAbis = listOf("arm64-v8a", "armeabi-v7a", "x86_64")
val workspaceRoot = rootProject.projectDir.parentFile
val nativeOutputDir = layout.projectDirectory.dir("src/main/jniLibs")

val buildAlvenqisNative by tasks.registering(Exec::class) {
    group = "build"
    description = "Builds the Rust Alvenqis mobile core for every packaged Android ABI."
    workingDir = workspaceRoot
    inputs.files(
        fileTree(workspaceRoot.resolve("alvenqis-mobile-core/src")),
        fileTree(workspaceRoot.resolve("alvenqis-core/src")),
        fileTree(workspaceRoot.resolve("alvenqis-core/native")),
        workspaceRoot.resolve("alvenqis-core/build.rs"),
        workspaceRoot.resolve("alvenqis-mobile-core/Cargo.toml"),
        workspaceRoot.resolve("alvenqis-core/Cargo.toml"),
        workspaceRoot.resolve("Cargo.lock")
    )
    outputs.dir(nativeOutputDir)

    if (System.getProperty("os.name").startsWith("Windows", ignoreCase = true)) {
        commandLine(
            "powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
            rootProject.projectDir.resolve("build-native.ps1").absolutePath
        )
    } else {
        commandLine("bash", rootProject.projectDir.resolve("build-native.sh").absolutePath)
    }
}

android {
    namespace = "network.alvenqis.mobile"
    compileSdk = 35
    ndkVersion = "29.0.14206865"
    defaultConfig {
        applicationId = "network.alvenqis.mobile"
        // Android 12 (API 31)+ - product floor for modern keystore / privacy APIs.
        minSdk = 31
        targetSdk = 35
        versionCode = 1000000
        versionName = "1.0.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk { abiFilters += alvenqisNativeAbis }
        buildConfigField("String", "PUBLIC_RPC", "\"https://rpcnode.dohotstudio.com\"")
        buildConfigField("String", "PRODUCT_LINE", "\"1.0.0\"")
    }
    buildFeatures { compose = true; buildConfig = true }
    packaging { jniLibs.useLegacyPackaging = false }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

tasks.named("preBuild") { dependsOn(buildAlvenqisNative) }

tasks.register("verifyDebugNativeLibraries") {
    group = "verification"
    description = "Rejects a debug APK that does not contain the Alvenqis Rust library for every supported ABI."
    dependsOn("assembleDebug")
    val apk = layout.buildDirectory.file("outputs/apk/debug/app-debug.apk")
    inputs.file(apk)
    doLast {
        val apkFile = apk.get().asFile
        check(apkFile.isFile) { "Android APK was not produced: $apkFile" }
        ZipFile(apkFile).use { zip ->
            alvenqisNativeAbis.forEach { abi ->
                listOf(
                    "lib/$abi/libalvenqis_mobile_core.so",
                    "lib/$abi/libc++_shared.so",
                ).forEach { entryName ->
                    val entry = zip.getEntry(entryName)
                        ?: error("APK is missing mandatory native library $entryName")
                    check(entry.size > 4) { "APK native library is empty: $entryName" }
                    zip.getInputStream(entry).use { input ->
                        val elf = input.readNBytes(4)
                        check(elf.contentEquals(byteArrayOf(0x7f, 0x45, 0x4c, 0x46))) {
                            "APK native library is not an ELF shared object: $entryName"
                        }
                    }
                }
            }
        }
        logger.lifecycle("Verified Alvenqis native libraries in ${apkFile.name}: ${alvenqisNativeAbis.joinToString()}")
    }
}

dependencies {
    implementation(platform("androidx.compose:compose-bom:2026.04.01"))
    implementation("androidx.activity:activity-compose:1.10.1")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.7")
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.biometric:biometric:1.1.0")
    implementation("androidx.fragment:fragment-ktx:1.8.6")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.10.1")
    implementation("org.json:json:20250107")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
    debugImplementation("androidx.compose.ui:ui-tooling")
}
