plugins {
    id("fabric-loom") version "1.7.4"
    id("com.github.johnrengelman.shadow") version "8.1.1"
}

val minecraftVersion = "1.21.1"
val yarnMappings = "1.21.1+build.3"
val fabricLoaderVersion = "0.16.5"
val fabricApiVersion = "0.102.0+1.21.1"

dependencies {
    minecraft("com.mojang:minecraft:$minecraftVersion")
    mappings("net.fabricmc:yarn:$yarnMappings:v2")
    modImplementation("net.fabricmc:fabric-loader:$fabricLoaderVersion")
    modImplementation("net.fabricmc.fabric-api:fabric-api:$fabricApiVersion")
    
    // Core module
    implementation(project(":core"))
    shadow(project(":core"))
}

tasks.processResources {
    inputs.property("version", project.version)
    
    filesMatching("fabric.mod.json") {
        expand("version" to project.version)
    }
}

java {
    withSourcesJar()
    sourceCompatibility = JavaVersion.VERSION_21
    targetCompatibility = JavaVersion.VERSION_21
}

tasks.jar {
    archiveClassifier.set("")
}

tasks {
    shadowJar {
        archiveClassifier.set("dev")
        configurations = listOf(project.configurations.shadow.get())
    }
    
    remapJar {
        dependsOn(shadowJar)
        inputFile.set(shadowJar.get().archiveFile)
        archiveClassifier.set("")
    }
    
    build {
        dependsOn(remapJar)
    }
}