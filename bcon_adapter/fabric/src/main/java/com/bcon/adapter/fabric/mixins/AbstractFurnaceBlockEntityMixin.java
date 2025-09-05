package com.bcon.adapter.fabric.mixins;

import com.bcon.adapter.core.events.ItemData;
import com.bcon.adapter.core.events.Location;
import com.bcon.adapter.fabric.FabricEventBridge;
import net.minecraft.block.entity.AbstractFurnaceBlockEntity;
import net.minecraft.item.ItemStack;
import net.minecraft.util.math.BlockPos;
import net.minecraft.world.World;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/**
 * Mixin for AbstractFurnaceBlockEntity to capture furnace events
 */
@Mixin(AbstractFurnaceBlockEntity.class)
public class AbstractFurnaceBlockEntityMixin {

    /**
     * Inject into the tick method to detect furnace state changes
     */
    @Inject(method = {"tick", "method_31652"}, at = @At("HEAD"), require = 0)
    private static void onTick(World world, BlockPos pos, net.minecraft.block.BlockState state, AbstractFurnaceBlockEntity blockEntity, CallbackInfo ci) {
        if (world.isClient()) {
            return;
        }
        
        // Track furnace operations - this is a simplified example
        // In a real implementation, you'd need to track state changes
        
        try {
            // Access furnace inventory through reflection or accessible methods
            // This is a basic framework - detailed implementation would require
            // deeper integration with the furnace's internal state
            
            Location location = new Location(
                pos.getX(), 
                pos.getY(), 
                pos.getZ(), 
                world.getRegistryKey().getValue().toString(),
                0f,
                0f
            );
            
            // Example: Detect if furnace is actively smelting
            // You would need to access burnTime, cookTime, cookTimeTotal fields
            // This requires careful field mapping and access
            
        } catch (Exception e) {
            // Silently handle any access issues
        }
    }

    /**
     * Inject into inventory change detection
     * This would fire when items are added/removed from furnace
     */
    @Inject(method = {"setStack", "method_5447"}, at = @At("HEAD"), require = 0)
    public void onSetStack(int slot, ItemStack stack, CallbackInfo ci) {
        AbstractFurnaceBlockEntity furnace = (AbstractFurnaceBlockEntity) (Object) this;
        
        if (furnace.getWorld() == null || furnace.getWorld().isClient()) {
            return;
        }
        
        try {
            Location location = new Location(
                furnace.getPos().getX(),
                furnace.getPos().getY(), 
                furnace.getPos().getZ(),
                furnace.getWorld().getRegistryKey().getValue().toString(),
                0f,
                0f
            );
            
            ItemData itemData = new ItemData(
                stack.getItem().toString(),
                stack.getCount(),
                stack.getName().getString(),
                null,
                stack.toString()
            );
            
            // Detect slot type and fire appropriate events
            if (slot == 0) { // Input slot
                // Could fire furnace_start_smelt when item is added
                FabricEventBridge.INSTANCE.fireFurnaceStartSmelt(location, itemData, 200); // Default cook time
            } else if (slot == 1) { // Fuel slot  
                // Could fire furnace_burn when fuel is added
                FabricEventBridge.INSTANCE.fireFurnaceBurn(location, itemData, 1600); // Default burn time
            } else if (slot == 2) { // Output slot
                // This is trickier - output is usually set by the furnace itself
                // Would need to detect when smelting completes
            }
            
        } catch (Exception e) {
            // Silently handle any issues
        }
    }
}