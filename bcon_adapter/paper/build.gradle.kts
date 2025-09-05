plugins {
    id("com.github.johnrengelman.shadow") version "8.1.1"
}

dependencies {
    // Paper API - using newer stable version
    compileOnly("io.papermc.paper:paper-api:1.21.1-R0.1-SNAPSHOT")
    
    // Core module  
    implementation(project(":core"))
}

tasks {    
    shadowJar {
        archiveClassifier.set("")
        // Basic shadow jar without complex transformations to avoid ASM Java 21 issues
    }
    
    build {
        dependsOn(shadowJar)
    }
}

