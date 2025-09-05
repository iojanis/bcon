package com.bcon.adapter.fabric.mixins;

import com.bcon.adapter.core.events.EntityData;
import com.bcon.adapter.core.events.Location;
import com.bcon.adapter.core.events.PlayerData;
import com.bcon.adapter.fabric.FabricEventBridge;
import net.minecraft.entity.passive.AnimalEntity;
import net.minecraft.entity.player.PlayerEntity;
import net.minecraft.server.network.ServerPlayerEntity;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/**
 * Mixin for AnimalEntity to capture breeding events
 */
@Mixin(AnimalEntity.class)
public class AnimalEntityMixin {

    /**
     * Inject into lovePlayer method to detect love mode entry
     */
    @Inject(method = {"lovePlayer", "method_6485"}, at = @At("HEAD"), require = 0)
    private void onLovePlayer(PlayerEntity player, CallbackInfo ci) {
        AnimalEntity animal = (AnimalEntity) (Object) this;
        
        if (animal.getWorld().isClient() || !(player instanceof ServerPlayerEntity)) {
            return;
        }
        
        try {
            EntityData entityData = new EntityData(
                animal.getUuidAsString(),
                animal.getType().toString(),
                new Location(
                    animal.getX(),
                    animal.getY(),
                    animal.getZ(),
                    animal.getWorld().getRegistryKey().getValue().toString(),
                    0f,
                    0f
                ),
                animal.hasCustomName() ? animal.getCustomName().getString() : null
            );
            
            PlayerData playerData = new PlayerData(
                player.getUuidAsString(),
                player.getName().getString(),
                new Location(
                    player.getX(),
                    player.getY(),
                    player.getZ(),
                    player.getWorld().getRegistryKey().getValue().toString(),
                    player.getYaw(),
                    player.getPitch()
                ),
                player.getHealth(),
                player.getMaxHealth(),
                player.experienceLevel,
                "SURVIVAL"
            );
            
            FabricEventBridge.INSTANCE.fireEntityEnterLoveMode(entityData, playerData);
            
        } catch (Exception e) {
            // Silently handle any issues
        }
    }

    /**
     * Inject into breed method to detect breeding start
     */
    @Inject(method = {"breed", "method_6474"}, at = @At("HEAD"), require = 0)
    private void onBreed(CallbackInfoReturnable<AnimalEntity> cir) {
        AnimalEntity animal = (AnimalEntity) (Object) this;
        
        if (animal.getWorld().isClient()) {
            return;
        }
        
        try {
            EntityData motherData = new EntityData(
                animal.getUuidAsString(),
                animal.getType().toString(),
                new Location(
                    animal.getX(),
                    animal.getY(),
                    animal.getZ(),
                    animal.getWorld().getRegistryKey().getValue().toString(),
                    0f,
                    0f
                ),
                animal.hasCustomName() ? animal.getCustomName().getString() : null
            );
            
            // Try to get the breeding player
            PlayerData breederData = null;
            if (animal.getLovingPlayer() instanceof ServerPlayerEntity breeder) {
                breederData = new PlayerData(
                    breeder.getUuidAsString(),
                    breeder.getName().getString(),
                    new Location(
                        breeder.getX(),
                        breeder.getY(),
                        breeder.getZ(),
                        animal.getWorld().getRegistryKey().getValue().toString(),
                        breeder.getYaw(),
                        breeder.getPitch()
                    ),
                    breeder.getHealth(),
                    breeder.getMaxHealth(),
                    breeder.experienceLevel,
                    "SURVIVAL"
                );
            }
            
            // Since we can't access the other breeding animal without method parameters,
            // we'll just fire with the one we have
            FabricEventBridge.INSTANCE.fireEntityStartBreeding(motherData, null, breederData, null);
            
        } catch (Exception e) {
            // Silently handle any issues
        }
    }
}