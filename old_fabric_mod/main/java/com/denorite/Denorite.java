package com.denorite;

import com.denorite.DenoriteProtocol;
import com.denorite.DenoriteProtocol.Message;
import com.google.gson.*;
import net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking;
import net.minecraft.block.*;
import net.minecraft.util.Identifier;
import net.fabricmc.fabric.api.entity.event.v1.ServerEntityCombatEvents;
import net.fabricmc.fabric.api.entity.event.v1.ServerPlayerEvents;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerWorldEvents;
import net.minecraft.entity.projectile.ProjectileEntity;
import net.minecraft.item.ItemStack;
import net.minecraft.screen.ScreenHandler;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.minecraft.advancement.AdvancementProgress;
import net.minecraft.advancement.AdvancementEntry;
import net.minecraft.server.network.ServerPlayerEntity;
import net.fabricmc.api.ModInitializer;
import net.fabricmc.fabric.api.entity.event.v1.*;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerLifecycleEvents;
import net.fabricmc.fabric.api.networking.v1.ServerPlayConnectionEvents;
import net.fabricmc.fabric.api.event.player.PlayerBlockBreakEvents;
import net.fabricmc.fabric.api.event.player.UseEntityCallback;
import net.fabricmc.fabric.api.event.player.AttackEntityCallback;
import net.fabricmc.fabric.api.event.player.UseItemCallback;
import net.fabricmc.fabric.api.event.player.UseBlockCallback;
import net.fabricmc.fabric.api.event.player.AttackBlockCallback;
import net.fabricmc.fabric.api.message.v1.ServerMessageEvents;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.command.CommandOutput;
import net.minecraft.server.command.ServerCommandSource;
import net.minecraft.server.world.ServerWorld;
import net.minecraft.text.Text;
import net.minecraft.util.ActionResult;
import net.minecraft.util.TypedActionResult;
import net.minecraft.util.math.BlockPos;
import net.minecraft.entity.Entity;
import net.minecraft.entity.damage.DamageSource;
import net.minecraft.util.math.Vec2f;
import net.minecraft.util.math.Vec3d;
import net.minecraft.world.World;
import net.minecraft.world.chunk.Chunk;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import net.minecraft.entity.LivingEntity;
import net.minecraft.entity.damage.DamageTracker;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.WebSocket;
import java.nio.ByteBuffer;
import java.util.concurrent.*;
import java.util.function.Supplier;

public class Denorite implements ModInitializer {
	public static final String MOD_ID = "Denorite";
	public static final Logger LOGGER = LoggerFactory.getLogger(MOD_ID);
	private static final StringBuffer messageBuffer = new StringBuffer();
	private static final Object bufferLock = new Object();

	private static WebSocket webSocket;
	private static final Gson gson = new Gson();
	static MinecraftServer server;
	private static boolean strictMode = true;
	private static final int RECONNECT_DELAY = 1000;
	private static boolean useBinaryProtocol = false; // Toggle for binary protocol

	private DenoriteConfig config;
	private static Block lastInteractedBlock = null;
	private static BlockPos lastInteractedPos = null;

	private static final ExecutorService executorService =
			Executors.newFixedThreadPool(Runtime.getRuntime().availableProcessors());

	// Method to submit tasks
	public static CompletableFuture<Void> submitTask(Runnable task) {
		return CompletableFuture.runAsync(task, executorService);
	}

	// Method to submit tasks with results
	public static <T> CompletableFuture<T> submitTask(Supplier<T> task) {
		return CompletableFuture.supplyAsync(task, executorService);
	}

	@Override
	public void onInitialize() {
		LOGGER.info("Initializing Denorite");
		config = new DenoriteConfig();
		initializeWebSocket();
		DynamicCommandHandler.initialize();
		BlueMapIntegration.initialize();
		ServerLifecycleEvents.SERVER_STARTING.register(this::setServer);
		ServerLifecycleEvents.SERVER_STOPPING.register(this::unsetServer);
		registerAllEvents();
	}

	private void setServer(MinecraftServer minecraftServer) {
		server = minecraftServer;
		FileSystemHandler.initialize(server);
	}

	private void unsetServer(MinecraftServer minecraftServer) {
		server = null;
	}

	private void initializeWebSocket() {
		connectWebSocket();
	}

	private void connectWebSocket() {
		try {
			HttpClient client = HttpClient.newHttpClient();
			String origin = config.getOrigin();
			if (origin == null || origin.isEmpty()) {
				throw new IllegalStateException("Origin must be set in the configuration");
			}

			CompletableFuture<WebSocket> ws = client.newWebSocketBuilder()
					.header("Authorization", "Bearer " + config.getJwtToken())
					.header("Origin", origin)
					.buildAsync(URI.create(config.getServerUrl()), new WebSocket.Listener() {
						@Override
						public void onOpen(WebSocket webSocket) {
							LOGGER.info("Connected to Denorite Server");
							Denorite.webSocket = webSocket;
							DynamicCommandHandler.handleReconnect();
							WebSocket.Listener.super.onOpen(webSocket);
						}

						@Override
						public CompletionStage<?> onText(WebSocket webSocket, CharSequence data, boolean last) {
							if (!useBinaryProtocol) {
								synchronized (bufferLock) {
									messageBuffer.append(data);

									if (last) {
										try {
											String completeMessage = messageBuffer.toString();
											handleIncomingMessage(completeMessage);
										} catch (Exception e) {
											LOGGER.error("Error processing complete message: " + e.getMessage());
											LOGGER.error("Message content: " + messageBuffer.toString());
										} finally {
											messageBuffer.setLength(0);
										}
									}
								}
							}
							return WebSocket.Listener.super.onText(webSocket, data, last);
						}

						@Override
						public CompletionStage<?> onBinary(WebSocket webSocket, ByteBuffer data, boolean last) {
							if (useBinaryProtocol) {
								try {
									byte[] bytes = new byte[data.remaining()];
									data.get(bytes);
									handleBinaryMessage(bytes);
								} catch (Exception e) {
									LOGGER.error("Error processing binary message: " + e.getMessage());
								}
							}
							return WebSocket.Listener.super.onBinary(webSocket, data, last);
						}

						@Override
						public CompletionStage<?> onClose(WebSocket webSocket, int statusCode, String reason) {
							LOGGER.warn("WebSocket closed: " + statusCode + " " + reason);
							handleDisconnect();
							return WebSocket.Listener.super.onClose(webSocket, statusCode, reason);
						}

						@Override
						public void onError(WebSocket webSocket, Throwable error) {
							LOGGER.error("WebSocket error: " + error.getMessage());
							handleDisconnect();
							WebSocket.Listener.super.onError(webSocket, error);
						}
					});

			try {
				ws.get(5, TimeUnit.SECONDS);
			} catch (Exception e) {
				LOGGER.error("Failed to connect to Denorite: " + e.getMessage());
				handleDisconnect();
			}
		} catch (Exception e) {
			LOGGER.error("Failed to initialize WebSocket: " + e.getMessage());
			handleDisconnect();
		}
	}

	private void handleDisconnect() {
		webSocket = null;
		if (config.isStrictMode()) {
			LOGGER.error("WebSocket disconnected in strict mode. Shutting down server.");
			if (server != null) {
				server.stop(false);
			}
		} else {
			LOGGER.warn("WebSocket disconnected. Attempting to reconnect in " + (RECONNECT_DELAY / 1000) + " seconds.");
			CompletableFuture.delayedExecutor(RECONNECT_DELAY, TimeUnit.MILLISECONDS).execute(this::connectWebSocket);
		}
	}

	public static void sendToTypeScript(String eventType, JsonObject data) {
		if (webSocket != null) {
			if (useBinaryProtocol) {
				// Convert to binary protocol message
				Message msg = DenoriteProtocol.createEventFromJson(eventType, data);
				webSocket.sendBinary(ByteBuffer.wrap(msg.serialize()), true);
			} else {
				// Legacy JSON format
				JsonObject jsonMessage = new JsonObject();
				jsonMessage.addProperty("eventType", eventType);
				jsonMessage.add("data", data);
				String message = jsonMessage.toString();
				webSocket.sendText(message, true);
			}
		}
	}

	private void handleBinaryMessage(byte[] message) {
		try {
			Message msg = DenoriteProtocol.parseMessage(message);
			switch (msg.getType()) {
				case DenoriteProtocol.MESSAGE_TYPE_REQUEST:
					handleBinaryRequest(msg);
					break;
				case DenoriteProtocol.MESSAGE_TYPE_RESPONSE:
					handleBinaryResponse(msg);
					break;
				case DenoriteProtocol.MESSAGE_TYPE_ERROR:
					handleBinaryError(msg);
					break;
				case DenoriteProtocol.MESSAGE_TYPE_PING:
					sendPong(msg.getId());
					break;
			}
		} catch (Exception e) {
			LOGGER.error("Error handling binary message: " + e.getMessage());
		}
	}

	private void handleBinaryRequest(Message msg) {
		// Extract request data
		DenoriteProtocol.Request request = msg.getRequest();

		// Process request based on category
		CompletableFuture.runAsync(() -> {
			try {
				Message response = switch (request.getCategory()) {
					case DenoriteProtocol.CATEGORY_COMMAND -> executeCommandBinary(request);
					case DenoriteProtocol.CATEGORY_CHAT -> broadcastMessageBinary(request);
					case DenoriteProtocol.CATEGORY_FILES -> handleFileCommandBinary(request);
					default -> DenoriteProtocol.createErrorResponse(msg.getId(), "Unknown request category");
				};

				if (webSocket != null) {
					webSocket.sendBinary(ByteBuffer.wrap(response.serialize()), true);
				}
			} catch (Exception e) {
				LOGGER.error("Error processing binary request: " + e.getMessage());
				if (webSocket != null) {
					Message errorResponse = DenoriteProtocol.createErrorResponse(msg.getId(), e.getMessage());
					webSocket.sendBinary(ByteBuffer.wrap(errorResponse.serialize()), true);
				}
			}
		});
	}

	private Message executeCommandBinary(DenoriteProtocol.Request request) {
		String command = request.getString("command");
		if (server != null) {
			try {
				StringBuilder output = new StringBuilder();
				ServerCommandSource source = server.getCommandSource().withOutput(new CommandOutput() {
					@Override
					public void sendMessage(Text message) {
						output.append(message.getString()).append("\n");
					}

					@Override
					public boolean shouldReceiveFeedback() {
						return true;
					}

					@Override
					public boolean shouldTrackOutput() {
						return true;
					}

					@Override
					public boolean shouldBroadcastConsoleToOps() {
						return false;
					}
				});

				server.getCommandManager().executeWithPrefix(source, command);
				return DenoriteProtocol.createCommandResponse(request.getId(), output.toString().trim());
			} catch (Exception e) {
				return DenoriteProtocol.createErrorResponse(request.getId(), e.getMessage());
			}
		}
		return DenoriteProtocol.createErrorResponse(request.getId(), "Server is null");
	}

	private Message broadcastMessageBinary(DenoriteProtocol.Request request) {
		String message = request.getString("message");
		if (server != null) {
			server.getPlayerManager().broadcast(Text.of(message), false);
			return DenoriteProtocol.createResponse(request.getId(), "Message broadcasted");
		}
		return DenoriteProtocol.createErrorResponse(request.getId(), "Server is null");
	}

	private Message handleFileCommandBinary(DenoriteProtocol.Request request) {
		try {
			String subcommand = request.getString("subcommand");
			JsonObject response = FileSystemHandler.handleFileCommand(
					subcommand,
					request.getJsonObject("arguments")
			);
			return DenoriteProtocol.createResponse(request.getId(), response.toString());
		} catch (Exception e) {
			return DenoriteProtocol.createErrorResponse(request.getId(), e.getMessage());
		}
	}

	private void sendPong(short id) {
		if (webSocket != null) {
			Message pong = DenoriteProtocol.createPong(id);
			webSocket.sendBinary(ByteBuffer.wrap(pong.serialize()), true);
		}
	}

	private void handleIncomingMessage(String message) {
		if (message == null || message.trim().isEmpty()) {
			LOGGER.warn("Received empty message");
			return;
		}

		try {
			JsonObject jsonMessage = gson.fromJson(message, JsonObject.class);
			if (!jsonMessage.has("id") || !jsonMessage.has("type")) {
				LOGGER.warn("Received malformed message without required fields: " + message);
				return;
			}

			// Process each message in its own thread
			CompletableFuture.runAsync(() -> {
				String id = jsonMessage.get("id").getAsString();
				String type = jsonMessage.get("type").getAsString();
				JsonObject response = new JsonObject();
				response.addProperty("id", id);

				try {
					String result = "";
					switch (type) {
						case "command":
							if (!jsonMessage.has("data")) {
								throw new IllegalArgumentException("Missing required 'data' field for command");
							}
							result = executeCommand(jsonMessage.get("data").getAsString());
							break;

						case "chat":
							if (!jsonMessage.has("data")) {
								throw new IllegalArgumentException("Missing required 'data' field for chat");
							}
							broadcastMessage(jsonMessage.get("data").getAsString());
							result = "Message broadcasted";
							break;

						case "bluemap":
							if (!jsonMessage.has("data") || !jsonMessage.get("data").isJsonObject()) {
								throw new IllegalArgumentException("Missing or invalid 'data' field for bluemap");
							}
							BlueMapIntegration.handleMarkerCommand(jsonMessage.get("data").getAsJsonObject());
							result = "Bluemap marker command executed";
							break;

						case "register_command":
							if (!jsonMessage.has("data") || !jsonMessage.get("data").isJsonObject()) {
								throw new IllegalArgumentException("Missing or invalid 'data' field for register_command");
							}
							DynamicCommandHandler.confirmReconnect();
							DynamicCommandHandler.registerCommand(jsonMessage.get("data").getAsJsonObject());
							result = "Command registered. Restart the server to apply changes.";
							break;

						case "unregister_command":
							if (!jsonMessage.has("data")) {
								throw new IllegalArgumentException("Missing required 'data' field for unregister_command");
							}
							DynamicCommandHandler.unregisterCommand(jsonMessage.get("data").getAsString());
							result = "Command unregistered. Restart the server to apply changes.";
							break;

						case "clear_commands":
							DynamicCommandHandler.clearCommands();
							result = "All custom commands cleared. Restart the server to apply changes.";
							break;

						case "files":
							if (!jsonMessage.has("subcommand")) {
								throw new IllegalArgumentException("Files command requires 'subcommand' field");
							}
							if (!jsonMessage.has("arguments") || !jsonMessage.get("arguments").isJsonObject()) {
								throw new IllegalArgumentException("Files command requires 'arguments' field");
							}
							result = FileSystemHandler.handleFileCommand(
									jsonMessage.get("subcommand").getAsString(),
									jsonMessage.get("arguments").getAsJsonObject()
							).toString();
							break;

						default:
							LOGGER.warn("Unknown message type: " + type);
							throw new IllegalArgumentException("Unknown message type: " + type);
					}
					LOGGER.info(result);
					response.addProperty("result", result);
				} catch (Exception e) {
					LOGGER.error("Error executing command: " + e.getMessage());
					response.addProperty("error", e.getMessage());
				}

				// Send response synchronously to ensure order
				synchronized (webSocket) {
					if (webSocket != null) {
						String responseString = response.toString();
						webSocket.sendText(responseString, true);
					}
				}
			});

		} catch (JsonSyntaxException e) {
			LOGGER.error("JSON parsing error: " + e.getMessage());
			LOGGER.error("Problematic message: " + message);
		} catch (Exception e) {
			LOGGER.error("Error handling message: " + e.getMessage());
			LOGGER.error("Message content: " + message);
		}
	}

	private void handleBinaryResponse(Message msg) {
		// Handle responses to our requests
		DenoriteProtocol.Response response = msg.getResponse();
		short requestId = response.getRequestId();

		// Look up and complete any pending request futures
		completePendingRequest(requestId, response);
	}

	private void handleBinaryError(Message msg) {
		DenoriteProtocol.ErrorResponse error = msg.getErrorResponse();
		LOGGER.error("Received error response: " + error.getMessage());

		// Complete any pending requests with the error
		completePendingRequest(error.getRequestId(), null, new RuntimeException(error.getMessage()));
	}

	private void completePendingRequest(short requestId, DenoriteProtocol.Response response) {
		completePendingRequest(requestId, response, null);
	}

	private void completePendingRequest(short requestId, DenoriteProtocol.Response response, Throwable error) {
		CompletableFuture<DenoriteProtocol.Response> future = DenoriteProtocol.getPendingRequest(requestId);
		if (future != null) {
			if (error != null) {
				future.completeExceptionally(error);
			} else {
				future.complete(response);
			}
		}
	}

	private String executeCommand(String command) {
		if (server != null) {
			try {
				StringBuilder output = new StringBuilder();
				ServerCommandSource source = server.getCommandSource().withOutput(new CommandOutput() {
					@Override
					public void sendMessage(Text message) {
						output.append(message.getString()).append("\n");
					}

					@Override
					public boolean shouldReceiveFeedback() {
						return true;
					}

					@Override
					public boolean shouldTrackOutput() {
						return true;
					}

					@Override
					public boolean shouldBroadcastConsoleToOps() {
						return false;
					}
				});

				server.getCommandManager().executeWithPrefix(source, command);
				return output.toString().trim();
			} catch (Exception e) {
				LOGGER.error("Error executing command: " + e.getMessage());
				return "Error: " + e.getMessage();
			}
		}
		return "Error: Server is null";
	}

	private void registerAllEvents() {
		registerServerEvents();
		registerPlayerEvents();
		registerEntityEvents();
		registerWorldEvents();
		registerChatEvents();
		registerProjectileEvents();
		registerAdvancementEvents();
		registerTradeEvents();
		registerRedstoneEvents();
	}

	private void registerServerEvents() {
		ServerLifecycleEvents.SERVER_STARTING.register(server -> sendToTypeScript("server_starting", null));
		ServerLifecycleEvents.SERVER_STARTED.register(server -> sendToTypeScript("server_started", null));
		ServerLifecycleEvents.SERVER_STOPPING.register(server -> sendToTypeScript("server_stopping", null));
		ServerLifecycleEvents.SERVER_STOPPED.register(server -> sendToTypeScript("server_stopped", null));
		//ServerTickEvents.START_SERVER_TICK.register((server) -> sendToTypeScript("server_tick_start", null));
		//ServerTickEvents.END_SERVER_TICK.register((server) -> sendToTypeScript("server_tick_end", null));

		ServerLifecycleEvents.BEFORE_SAVE.register((server, srt, str) ->
				sendToTypeScript("server_before_save", null));

		ServerLifecycleEvents.AFTER_SAVE.register((server, srt, str) ->
				sendToTypeScript("server_after_save", null));

		ServerLifecycleEvents.START_DATA_PACK_RELOAD.register((server, resourceManager) ->
				sendToTypeScript("data_pack_reload_start", null));

		ServerLifecycleEvents.END_DATA_PACK_RELOAD.register((server, resourceManager, success) -> {
			JsonObject data = new JsonObject();
			data.addProperty("success", success);
			sendToTypeScript("data_pack_reload_end", data);
		});

		ServerLifecycleEvents.SYNC_DATA_PACK_CONTENTS.register((player, joined) -> {
			JsonObject data = new JsonObject();
			data.addProperty("playerId", player.getUuidAsString());
			data.addProperty("playerName", player.getName().getString());
			data.addProperty("joined", joined);
			sendToTypeScript("data_pack_sync", data);
		});
	}

	private void registerPlayerEvents() {
		ServerPlayerEvents.AFTER_RESPAWN.register((oldPlayer, newPlayer, alive) -> {
			sendToTypeScript("player_respawned", serializePlayerRespawn(oldPlayer, newPlayer, alive));
		});

		ServerPlayConnectionEvents.JOIN.register((handler, sender, server) ->
				sendToTypeScript("player_joined", serializePlayer(handler.getPlayer())));

		ServerPlayConnectionEvents.DISCONNECT.register((handler, server) ->
				sendToTypeScript("player_left", serializePlayer(handler.getPlayer())));

		ServerPlayConnectionEvents.INIT.register((handler, server) -> {
			JsonObject data = new JsonObject();
			data.addProperty("playerId", handler.player.getUuidAsString());
			data.addProperty("playerName", handler.player.getName().getString());
			sendToTypeScript("player_connection_init", data);
		});

		PlayerBlockBreakEvents.BEFORE.register((world, player, pos, state, blockEntity) -> {
			sendToTypeScript("player_break_block_before", serializeBlockEvent((ServerPlayerEntity)player, pos, state));
			return true;
		});

		PlayerBlockBreakEvents.AFTER.register((world, player, pos, state, blockEntity) ->
				sendToTypeScript("player_break_block_after", serializeBlockEvent((ServerPlayerEntity)player, pos, state)));
	}

	private void registerEntityEvents() {
		ServerEntityCombatEvents.AFTER_KILLED_OTHER_ENTITY.register((world, entity, killedEntity) -> {
			if (killedEntity instanceof ServerPlayerEntity) {
				sendToTypeScript("player_death", serializePlayerDeath((ServerPlayerEntity) killedEntity, killedEntity.getRecentDamageSource()));
			} else {
				sendToTypeScript("entity_death", serializeEntityDeath(entity, killedEntity));
			}
		});
	}

	private void registerWorldEvents() {
		ServerWorldEvents.LOAD.register((server, world) ->
				sendToTypeScript("world_load", serializeWorld(world)));

		ServerWorldEvents.UNLOAD.register((server, world) ->
				sendToTypeScript("world_unload", serializeWorld(world)));
	}

	private void registerChatEvents() {
		ServerMessageEvents.CHAT_MESSAGE.register((message, sender, params) -> {
			sendToTypeScript("player_chat", serializeChat(sender.getCommandSource(), message.getContent().getString()));
		});

		ServerMessageEvents.COMMAND_MESSAGE.register((message, sender, params) -> {
			JsonObject data = new JsonObject();
			if (sender.getPlayer() != null) {
				data.addProperty("playerId", sender.getPlayer().getUuidAsString());
				data.addProperty("playerName", sender.getPlayer().getName().getString());
			}
			data.addProperty("message", message.getContent().getString());
			sendToTypeScript("command_message", data);
		});
	}

	private void registerProjectileEvents() {
		ServerEntityCombatEvents.AFTER_KILLED_OTHER_ENTITY.register((world, entity, killed) -> {
			if (entity instanceof ProjectileEntity projectile) {
				Entity owner = projectile.getOwner();
				JsonObject data = new JsonObject();
				data.addProperty("projectileType", projectile.getType().toString());
				if (owner != null) {
					data.addProperty("ownerId", owner.getUuidAsString());
					data.addProperty("ownerType", owner.getType().toString());
				}
				data.add("target", serializeEntity(killed));
				sendToTypeScript("projectile_kill", data);
			}
		});
	}

	private void registerAdvancementEvents() {
		ServerPlayerEvents.AFTER_RESPAWN.register((oldPlayer, newPlayer, alive) -> {
			for (AdvancementEntry advancement : server.getAdvancementLoader().getAdvancements()) {
				AdvancementProgress progress = newPlayer.getAdvancementTracker().getProgress(advancement);
				if (progress.isDone()) {
					sendToTypeScript("advancement_complete", serializeAdvancement(newPlayer, advancement));
				}
			}
		});
	}

	private void registerTradeEvents() {
		UseEntityCallback.EVENT.register((player, world, hand, entity, hitResult) -> {
			if (entity instanceof net.minecraft.village.Merchant) {
				JsonObject data = new JsonObject();
				data.addProperty("playerId", player.getUuidAsString());
				data.addProperty("merchantId", entity.getUuidAsString());
				data.addProperty("merchantType", entity.getType().toString());
				sendToTypeScript("merchant_interaction", data);
			}
			return ActionResult.PASS;
		});
	}

	private void registerRedstoneEvents() {
		AttackBlockCallback.EVENT.register((player, world, hand, pos, direction) -> {
			BlockState state = world.getBlockState(pos);
			if (state.getBlock() instanceof RedstoneWireBlock ||
					state.getBlock() instanceof AbstractRedstoneGateBlock) {
				JsonObject data = new JsonObject();
				data.addProperty("x", pos.getX());
				data.addProperty("y", pos.getY());
				data.addProperty("z", pos.getZ());
				data.addProperty("power", state.get(RedstoneWireBlock.POWER));
				sendToTypeScript("redstone_update", data);
			}
			return ActionResult.PASS;
		});
	}

	private JsonObject serializePlayer(ServerPlayerEntity player) {
		JsonObject data = new JsonObject();
		data.addProperty("playerId", player.getUuidAsString());
		data.addProperty("playerName", player.getName().getString());
		data.addProperty("x", player.getX());
		data.addProperty("y", player.getY());
		data.addProperty("z", player.getZ());
		data.addProperty("dimension", player.getWorld().getRegistryKey().getValue().toString());
		return data;
	}

	private JsonObject serializePlayerRespawn(ServerPlayerEntity oldPlayer, ServerPlayerEntity newPlayer, boolean alive) {
		JsonObject data = new JsonObject();
		data.addProperty("playerId", newPlayer.getUuidAsString());
		data.addProperty("playerName", newPlayer.getName().getString());
		data.addProperty("alive", alive);
		data.addProperty("x", newPlayer.getX());
		data.addProperty("y", newPlayer.getY());
		data.addProperty("z", newPlayer.getZ());
		data.addProperty("dimension", newPlayer.getWorld().getRegistryKey().getValue().toString());
		return data;
	}

	private JsonObject serializeBlockEvent(ServerPlayerEntity player, BlockPos pos, BlockState state) {
		JsonObject data = new JsonObject();
		data.addProperty("playerId", player.getUuidAsString());
		data.addProperty("x", pos.getX());
		data.addProperty("y", pos.getY());
		data.addProperty("z", pos.getZ());
		data.addProperty("block", state.getBlock().toString());
		return data;
	}

	private JsonObject serializePlayerDeath(ServerPlayerEntity player, DamageSource recentDamageSource) {
		JsonObject data = serializePlayer(player);
		DamageTracker damageTracker = player.getDamageTracker();
		Text deathMessage = damageTracker.getDeathMessage();

		if (deathMessage != null) {
			data.addProperty("deathMessage", deathMessage.getString());
		}

		Entity attacker = player.getAttacker();
		if (attacker != null) {
			data.addProperty("attackerId", attacker.getUuidAsString());
			data.addProperty("attackerType", attacker.getType().toString());
		}

		return data;
	}

	private JsonObject serializeEntityDeath(Entity killer, LivingEntity killedEntity) {
		JsonObject data = new JsonObject();
		data.add("killedEntity", serializeEntity(killedEntity));
		if (killer != null) {
			data.add("killer", serializeEntity(killer));
		}

		DamageTracker damageTracker = killedEntity.getDamageTracker();
		Text deathMessage = damageTracker.getDeathMessage();

		if (deathMessage != null) {
			data.addProperty("deathMessage", deathMessage.getString());
		}

		return data;
	}

	private JsonObject serializeWorld(ServerWorld world) {
		JsonObject data = new JsonObject();
		data.addProperty("dimensionKey", world.getRegistryKey().getValue().toString());
		data.addProperty("time", world.getTime());
		data.addProperty("difficultyLevel", world.getDifficulty().getName());
		return data;
	}

	private JsonObject serializeChat(ServerCommandSource sender, String message) {
		JsonObject data = new JsonObject();
		ServerPlayerEntity player = sender.getPlayer();
		if (player != null) {
			data.addProperty("playerId", player.getUuidAsString());
			data.addProperty("playerName", player.getName().getString());
		}
		data.addProperty("message", message);
		return data;
	}

	private JsonObject serializeEntity(Entity entity) {
		JsonObject data = new JsonObject();
		data.addProperty("entityId", entity.getUuidAsString());
		data.addProperty("entityType", entity.getType().toString());
		data.addProperty("x", entity.getX());
		data.addProperty("y", entity.getY());
		data.addProperty("z", entity.getZ());
		return data;
	}

	private JsonObject serializeAdvancement(ServerPlayerEntity player, AdvancementEntry advancement) {
		JsonObject data = new JsonObject();
		data.addProperty("playerId", player.getUuidAsString());
		data.addProperty("playerName", player.getName().getString());
		data.addProperty("advancementId", advancement.id().toString());
		data.addProperty("title", advancement.value().display().toString());
		return data;
	}

	private void broadcastMessage(String message) {
		if (server != null) {
			server.getPlayerManager().broadcast(Text.of(message), false);
		}
	}

}
