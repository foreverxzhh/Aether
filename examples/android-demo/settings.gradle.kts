pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolution {
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "AetherDemo"
include(":app")

// 引用 SDK 本地模块
include(":aether-sdk")
project(":aether-sdk").projectDir = file("../../sdks/android")
