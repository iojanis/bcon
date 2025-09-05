package com.bcon.adapter.fabric.mixins;

import com.bcon.adapter.core.events.EntityData;
import com.bcon.adapter.core.events.FishHookData;
import com.bcon.adapter.core.events.Location;
import com.bcon.adapter.core.events.PlayerData;
import com.bcon.adapter.fabric.FabricEventBridge;
import net.minecraft.entity.Entity;
import net.minecraft.entity.player.PlayerEntity;
import net.minecraft.entity.projectile.FishingBobberEntity;
import net.minecraft.server.network.ServerPlayerEntity;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/**
 * Mixin for FishingBobberEntity to capture fishing events
 */
@Mixin(FishingBobberEntity.class)
public class FishingBobberEntityMixin {


    /**
     * Inject into use method to detect fishing interactions
     */
    @Inject(method = {"use", "method_7218"}, at = @At("RETURN"), require = 0)
    private void onUse(CallbackInfoReturnable<Integer> cir) {
        FishingBobberEntity bobber = (FishingBobberEntity) (Object) this;
        PlayerEntity owner = ((FishingBobberEntity) (Object) this).getPlayerOwner();
        
        if (bobber.getWorld().isClient() || !(owner instanceof ServerPlayerEntity)) {
            return;
        }
        
        try {
            ServerPlayerEntity player = (ServerPlayerEntity) owner;
            int result = cir.getReturnValue();
            
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
            
            FishHookData hookData = new FishHookData(
                new Location(
                    bobber.getX(),
                    bobber.getY(),
                    bobber.getZ(),
                    bobber.getWorld().getRegistryKey().getValue().toString(),
                    0f,
                    0f
                ),
                bobber.isInOpenWater(),
                0 // waitTime would need reflection to access
            );
            
            // Interpret the result to determine what happened
            if (result == 1) {
                // Something was caught - could be fish or entity
                // We would need to check what was actually caught
                FabricEventBridge.INSTANCE.firePlayerFishingReelIn(playerData, hookData);
            } else if (result == 0) {
                // Nothing caught or failed attempt
                FabricEventBridge.INSTANCE.firePlayerFishEscape(playerData, hookData);
            }
            
        } catch (Exception e) {
            // Silently handle any issues
        }
    }

    /**
     * Inject into onRemoved to detect when bobber is removed from world
     */
    @Inject(method = {"onRemoved", "method_5650"}, at = @At("HEAD"), require = 0)
    private void onRemoved(CallbackInfo ci) {
        FishingBobberEntity bobber = (FishingBobberEntity) (Object) this;
        PlayerEntity owner = ((FishingBobberEntity) (Object) this).getPlayerOwner();
        
        if (bobber.getWorld().isClient() || !(owner instanceof ServerPlayerEntity)) {
            return;
        }
        
        try {
            ServerPlayerEntity player = (ServerPlayerEntity) owner;
            
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
            
            FishHookData hookData = new FishHookData(
                new Location(
                    bobber.getX(),
                    bobber.getY(),
                    bobber.getZ(),
                    bobber.getWorld().getRegistryKey().getValue().toString(),
                    0f,
                    0f
                ),
                bobber.isInOpenWater(),
                0
            );
            
            // Bobber removed - could be reel in or timeout
            FabricEventBridge.INSTANCE.firePlayerFishingReelIn(playerData, hookData);
            
        } catch (Exception e) {
            // Silently handle any issues
        }
    }
}