pluginManagement {
    repositories {
        gradlePluginPortal()
        maven("https://maven.fabricmc.net/")
        maven("https://repo.papermc.io/repository/maven-public/")
        maven("https://maven.quiltmc.org/repository/release/")
    }
}

rootProject.name = "Bcon"

include(":core")
include(":paper")
include(":fabric")
include(":folia")
