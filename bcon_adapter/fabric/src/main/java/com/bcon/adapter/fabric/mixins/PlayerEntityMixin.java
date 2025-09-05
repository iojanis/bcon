package com.bcon.adapter.fabric.mixins;

import com.bcon.adapter.core.events.ItemData;
import com.bcon.adapter.core.events.Location;
import com.bcon.adapter.core.events.PlayerData;
import com.bcon.adapter.fabric.FabricEventBridge;
import net.minecraft.entity.ItemEntity;
import net.minecraft.entity.player.PlayerEntity;
import net.minecraft.item.ItemStack;
import net.minecraft.server.network.ServerPlayerEntity;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/**
 * Mixin for PlayerEntity to capture player interaction events
 */
@Mixin(PlayerEntity.class)
public class PlayerEntityMixin {

    /**
     * Inject into dropItem method to detect item drops
     * Using multiple possible method signatures to handle mapping variations
     */
    @Inject(method = {"dropItem(Lnet/minecraft/item/ItemStack;ZZ)Lnet/minecraft/entity/ItemEntity;", "method_7174"}, at = @At("RETURN"), require = 0)
    private void onDropItem(ItemStack stack, boolean throwRandomly, boolean retainOwnership, CallbackInfoReturnable<ItemEntity> cir) {
        PlayerEntity player = (PlayerEntity) (Object) this;
        ItemEntity droppedItem = cir.getReturnValue();
        
        if (player.getWorld().isClient() || !(player instanceof ServerPlayerEntity) || droppedItem == null) {
            return;
        }
        
        try {
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
                "SURVIVAL" // Default game mode, would need reflection to get actual
            );
            
            ItemData itemData = new ItemData(
                stack.getItem().toString(),
                stack.getCount(),
                stack.getName().getString(),
                null, // lore not easily accessible
                stack.toString() // simplified NBT
            );
            
            Location location = new Location(
                droppedItem.getX(),
                droppedItem.getY(),
                droppedItem.getZ(),
                droppedItem.getWorld().getRegistryKey().getValue().toString(),
                0f,
                0f
            );
            
            FabricEventBridge.INSTANCE.firePlayerItemDrop(playerData, itemData, location);
            
        } catch (Exception e) {
            // Silently handle any issues
        }
    }
}