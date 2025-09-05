plugins {
    kotlin("jvm") version "1.9.20" apply false
    kotlin("plugin.serialization") version "1.9.20" apply false
}

allprojects {
    group = "sh.cou.bcon"
    version = "1.0.0"
    
    repositories {
        mavenCentral()
        maven("https://maven.fabricmc.net/")
        maven("https://repo.spigotmc.org/content/repositories/snapshots/")
        maven("https://hub.spigotmc.org/nexus/content/repositories/snapshots/")
        maven("https://maven.quiltmc.org/repository/release/")
        maven("https://repo.papermc.io/repository/maven-public/")
        maven("https://repo.codemc.io/repository/maven-public/")
        maven("https://jitpack.io")
    }
}

subprojects {
    apply(plugin = "org.jetbrains.kotlin.jvm")
    apply(plugin = "org.jetbrains.kotlin.plugin.serialization")

    dependencies {
        val implementation by configurations
        val compileOnly by configurations
        
        // Core dependencies
        implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.0")
        implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.7.3")
        
        // JSON (using Gson for now, can replace later)
        implementation("com.google.code.gson:gson:2.10.1")
        
        // WebSocket client
        implementation("org.java-websocket:Java-WebSocket:1.5.4")
        
        // Logging
        implementation("org.slf4j:slf4j-api:2.0.9")
        compileOnly("org.apache.logging.log4j:log4j-core:2.21.1")
    }

    tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile> {
        kotlinOptions {
            jvmTarget = "21"
        }
    }

    tasks.withType<JavaCompile> {
        options.release.set(21)
    }
}