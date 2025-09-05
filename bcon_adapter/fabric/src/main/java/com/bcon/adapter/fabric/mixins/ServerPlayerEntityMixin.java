package com.bcon.adapter.fabric.mixins;

import com.bcon.adapter.core.events.EntityData;
import com.bcon.adapter.core.events.Location;
import com.bcon.adapter.fabric.FabricEventBridge;
import net.minecraft.entity.Entity;
import net.minecraft.server.network.ServerPlayerEntity;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/**
 * Mixin for ServerPlayerEntity to capture server-side player events
 */
@Mixin(ServerPlayerEntity.class)
public class ServerPlayerEntityMixin {

    /**
     * Inject into startRiding method to detect entity mounting
     */
    @Inject(method = {"startRiding", "method_5873"}, at = @At("HEAD"), require = 0)
    private void onStartRiding(Entity entity, boolean force, CallbackInfoReturnable<Boolean> cir) {
        ServerPlayerEntity player = (ServerPlayerEntity) (Object) this;
        
        if (player.getWorld().isClient()) {
            return;
        }
        
        try {
            EntityData riderData = new EntityData(
                player.getUuidAsString(),
                player.getType().toString(),
                new Location(
                    player.getX(),
                    player.getY(),
                    player.getZ(),
                    player.getWorld().getRegistryKey().getValue().toString(),
                    player.getYaw(),
                    player.getPitch()
                ),
                player.hasCustomName() ? player.getCustomName().getString() : player.getName().getString()
            );
            
            EntityData mountData = new EntityData(
                entity.getUuidAsString(),
                entity.getType().toString(),
                new Location(
                    entity.getX(),
                    entity.getY(),
                    entity.getZ(),
                    entity.getWorld().getRegistryKey().getValue().toString(),
                    0f,
                    0f
                ),
                entity.hasCustomName() ? entity.getCustomName().getString() : null
            );
            
            FabricEventBridge.INSTANCE.fireEntityMount(riderData, mountData);
            
        } catch (Exception e) {
            // Silently handle any issues
        }
    }

    /**
     * Inject into stopRiding method to detect entity dismounting
     */
    @Inject(method = {"stopRiding", "method_5848"}, at = @At("HEAD"), require = 0)
    private void onStopRiding(CallbackInfo ci) {
        ServerPlayerEntity player = (ServerPlayerEntity) (Object) this;
        Entity vehicle = player.getVehicle();
        
        if (player.getWorld().isClient() || vehicle == null) {
            return;
        }
        
        try {
            EntityData riderData = new EntityData(
                player.getUuidAsString(),
                player.getType().toString(),
                new Location(
                    player.getX(),
                    player.getY(),
                    player.getZ(),
                    player.getWorld().getRegistryKey().getValue().toString(),
                    player.getYaw(),
                    player.getPitch()
                ),
                player.hasCustomName() ? player.getCustomName().getString() : player.getName().getString()
            );
            
            EntityData mountData = new EntityData(
                vehicle.getUuidAsString(),
                vehicle.getType().toString(),
                new Location(
                    vehicle.getX(),
                    vehicle.getY(),
                    vehicle.getZ(),
                    vehicle.getWorld().getRegistryKey().getValue().toString(),
                    0f,
                    0f
                ),
                vehicle.hasCustomName() ? vehicle.getCustomName().getString() : null
            );
            
            FabricEventBridge.INSTANCE.fireEntityDismount(riderData, mountData);
            
        } catch (Exception e) {
            // Silently handle any issues
        }
    }
}