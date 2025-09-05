package com.bcon.adapter.fabric

import com.bcon.adapter.core.events.*

/**
 * Bridge class to allow mixins to communicate with the Fabric adapter
 * Provides static access to event handling methods
 */
object FabricEventBridge {
    
    private var adapter: FabricBconAdapter? = null
    
    /**
     * Register the adapter instance for mixins to use
     */
    fun registerAdapter(fabricAdapter: FabricBconAdapter) {
        adapter = fabricAdapter
    }
    
    /**
     * Unregister the adapter instance
     */
    fun unregisterAdapter() {
        adapter = null
    }
    
    // Event firing methods for mixins
    
    fun firePlayerItemDrop(playerData: PlayerData, itemData: ItemData, location: Location) {
        adapter?.eventManager?.onPlayerItemDrop(playerData, itemData, location)
    }
    
    fun fireFurnaceSmelt(location: Location, source: ItemData, result: ItemData) {
        adapter?.eventManager?.onFurnaceSmelt(location, source, result)
    }
    
    fun fireFurnaceBurn(location: Location, fuel: ItemData, burnTime: Int) {
        adapter?.eventManager?.onFurnaceBurn(location, fuel, burnTime)
    }
    
    fun fireFurnaceStartSmelt(location: Location, source: ItemData, totalCookTime: Int) {
        adapter?.eventManager?.onFurnaceStartSmelt(location, source, totalCookTime)
    }
    
    fun fireEntityStartBreeding(mother: EntityData, father: EntityData, breeder: PlayerData?, offspring: EntityData?) {
        adapter?.eventManager?.onEntityStartBreeding(mother, father, breeder, offspring)
    }
    
    fun fireEntityEnterLoveMode(entity: EntityData, cause: PlayerData?) {
        adapter?.eventManager?.onEntityEnterLoveMode(entity, cause)
    }
    
    fun firePlayerFishingCast(player: PlayerData, hook: FishHookData) {
        adapter?.eventManager?.onPlayerFishingCast(player, hook)
    }
    
    fun firePlayerFishCaught(player: PlayerData, fish: EntityData, hook: FishHookData) {
        adapter?.eventManager?.onPlayerFishCaught(player, fish, hook)
    }
    
    fun firePlayerFishEscape(player: PlayerData, hook: FishHookData) {
        adapter?.eventManager?.onPlayerFishEscape(player, hook)
    }
    
    fun firePlayerFishingReelIn(player: PlayerData, hook: FishHookData) {
        adapter?.eventManager?.onPlayerFishingReelIn(player, hook)
    }
    
    fun fireEntityMount(rider: EntityData, mount: EntityData) {
        adapter?.eventManager?.onEntityMount(rider, mount)
    }
    
    fun fireEntityDismount(rider: EntityData, mount: EntityData) {
        adapter?.eventManager?.onEntityDismount(rider, mount)
    }
}