package com.denorite;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import io.netty.buffer.ByteBuf;
import io.netty.buffer.Unpooled;
import net.minecraft.util.math.BlockPos;

import java.nio.charset.StandardCharsets;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;

public class DenoriteProtocol {
    // Message Types
    public static final byte MESSAGE_TYPE_EVENT = 0x01;
    public static final byte MESSAGE_TYPE_REQUEST = 0x02;
    public static final byte MESSAGE_TYPE_RESPONSE = 0x03;
    public static final byte MESSAGE_TYPE_ERROR = 0x04;
    public static final byte MESSAGE_TYPE_PING = 0x05;
    public static final byte MESSAGE_TYPE_PONG = 0x06;

    // Categories
    public static final byte CATEGORY_PLAYER = 0x01;
    public static final byte CATEGORY_WORLD = 0x02;
    public static final byte CATEGORY_ENTITY = 0x03;
    public static final byte CATEGORY_BLOCK = 0x04;
    public static final byte CATEGORY_CHAT = 0x05;
    public static final byte CATEGORY_COMMAND = 0x06;
    public static final byte CATEGORY_FILES = 0x07;

    private static final byte PROTOCOL_VERSION = 0x01;
    private static final Map<Short, CompletableFuture<Response>> pendingRequests = new ConcurrentHashMap<>();
    private static short nextRequestId = 0;

    public static class Message {
        private final byte type;
        private final short id;
        private final ByteBuf payload;

        public Message(byte type, short id, ByteBuf payload) {
            this.type = type;
            this.id = id;
            this.payload = payload;
        }

        public byte getType() {
            return type;
        }

        public short getId() {
            return id;
        }

        public Request getRequest() {
            return new Request(id, payload);
        }

        public Response getResponse() {
            return new Response(id, payload);
        }

        public ErrorResponse getErrorResponse() {
            return new ErrorResponse(id, readString(payload));
        }

        public byte[] serialize() {
            ByteBuf buf = Unpooled.buffer();
            buf.writeByte(PROTOCOL_VERSION);
            buf.writeByte(type);
            buf.writeShort(id);
            buf.writeBytes(payload);

            byte[] result = new byte[buf.readableBytes()];
            buf.readBytes(result);
            return result;
        }
    }

    public static class Request {
        private final short id;
        private final ByteBuf data;

        public Request(short id, ByteBuf data) {
            this.id = id;
            this.data = data;
        }

        public short getId() {
            return id;
        }

        public byte getCategory() {
            return data.readByte();
        }

        public String getString(String key) {
            // Find the key in the buffer and return its string value
            return readString(data);
        }

        public JsonObject getJsonObject(String key) {
            // Find the key in the buffer and parse its JSON value
            String jsonStr = readString(data);
            return com.google.gson.JsonParser.parseString(jsonStr).getAsJsonObject();
        }
    }

    public static class Response {
        private final short requestId;
        private final String data;

        public Response(short requestId, ByteBuf data) {
            this.requestId = requestId;
            this.data = readString(data);
        }

        public short getRequestId() {
            return requestId;
        }

        public String getData() {
            return data;
        }
    }

    public static class ErrorResponse {
        private final short requestId;
        private final String message;

        public ErrorResponse(short requestId, String message) {
            this.requestId = requestId;
            this.message = message;
        }

        public short getRequestId() {
            return requestId;
        }

        public String getMessage() {
            return message;
        }
    }

    public static Message parseMessage(byte[] data) {
        ByteBuf buf = Unpooled.wrappedBuffer(data);
        byte version = buf.readByte();
        byte type = buf.readByte();
        short id = buf.readShort();
        return new Message(type, id, buf.retain());
    }

    public static Message createEventFromJson(String eventType, JsonObject data) {
        ByteBuf buf = Unpooled.buffer();
        writeString(buf, eventType);
        writeJson(buf, data);
        return new Message(MESSAGE_TYPE_EVENT, nextRequestId++, buf);
    }

    public static Message createErrorResponse(short requestId, String errorMessage) {
        ByteBuf buf = Unpooled.buffer();
        writeString(buf, errorMessage);
        return new Message(MESSAGE_TYPE_ERROR, requestId, buf);
    }

    public static Message createResponse(short requestId, String response) {
        ByteBuf buf = Unpooled.buffer();
        writeString(buf, response);
        return new Message(MESSAGE_TYPE_RESPONSE, requestId, buf);
    }

    public static Message createCommandResponse(short requestId, String output) {
        ByteBuf buf = Unpooled.buffer();
        writeString(buf, output);
        return new Message(MESSAGE_TYPE_RESPONSE, requestId, buf);
    }

    public static Message createPong(short pingId) {
        return new Message(MESSAGE_TYPE_PONG, pingId, Unpooled.buffer(0));
    }

    public static CompletableFuture<Response> getPendingRequest(short requestId) {
        return pendingRequests.remove(requestId);
    }

    private static String readString(ByteBuf buf) {
        int length = buf.readShort();
        byte[] bytes = new byte[length];
        buf.readBytes(bytes);
        return new String(bytes, StandardCharsets.UTF_8);
    }

    private static void writeString(ByteBuf buf, String str) {
        byte[] bytes = str.getBytes(StandardCharsets.UTF_8);
        buf.writeShort(bytes.length);
        buf.writeBytes(bytes);
    }

    private static void writeJson(ByteBuf buf, JsonObject json) {
        writeString(buf, json.toString());
    }
}
