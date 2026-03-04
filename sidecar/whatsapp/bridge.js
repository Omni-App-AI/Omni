#!/usr/bin/env node
/**
 * Omni WhatsApp Web Bridge
 *
 * JSON-RPC bridge over stdin/stdout for communicating with the Omni Rust runtime.
 * Uses Baileys to emulate WhatsApp Web protocol — free, no API costs.
 *
 * Protocol:
 *   → stdin:  {"id":1,"method":"connect","params":{"authDir":"..."}}
 *   ← stdout: {"event":"qr","data":{"qrImage":"data:image/png;base64,..."}}
 *   ← stdout: {"event":"connected","data":{"phone":"...","name":"..."}}
 *   ← stdout: {"event":"message","data":{...}}
 *   → stdin:  {"id":2,"method":"send","params":{"to":"+1...","text":"Hello"}}
 *   ← stdout: {"id":2,"result":{"messageId":"..."}}
 */

const { makeWASocket, useMultiFileAuthState, fetchLatestBaileysVersion, DisconnectReason } = require("@whiskeysockets/baileys");
const QRCode = require("qrcode");
const { createInterface } = require("readline");
const path = require("path");
const fs = require("fs");

let sock = null;
let authDir = null;
let reconnectAttempts = 0;
let wasEverConnected = false;
const MAX_RECONNECT_ATTEMPTS = 10;

// Send a JSON event to stdout (for Rust to read)
function emit(obj) {
  process.stdout.write(JSON.stringify(obj) + "\n");
}

// Send a response to a request
function respond(id, result) {
  emit({ id, result });
}

// Send an error response
function respondError(id, error) {
  emit({ id, error: String(error) });
}

// Send an event (no ID, just notification)
function emitEvent(event, data) {
  emit({ event, data });
}

// Handle incoming commands from stdin
async function handleCommand(line) {
  let msg;
  try {
    msg = JSON.parse(line.trim());
  } catch {
    return; // Ignore malformed JSON
  }

  const { id, method, params } = msg;

  try {
    switch (method) {
      case "connect":
        await handleConnect(id, params || {});
        break;
      case "send":
        await handleSend(id, params || {});
        break;
      case "disconnect":
        await handleDisconnect(id);
        break;
      case "logout":
        await handleLogout(id);
        break;
      case "status":
        respond(id, { connected: sock !== null });
        break;
      default:
        respondError(id, `Unknown method: ${method}`);
    }
  } catch (err) {
    respondError(id, err.message || String(err));
  }
}

async function handleConnect(id, params) {
  authDir = params.authDir || path.join(process.env.HOME || process.env.USERPROFILE || ".", ".omni", "channels", "whatsapp");

  wasEverConnected = false;

  // Clean up existing socket before creating a new one
  if (sock) {
    try {
      sock.ev.removeAllListeners();
      sock.end(undefined);
    } catch {
      // Ignore cleanup errors
    }
    sock = null;
  }

  // Ensure auth directory exists
  fs.mkdirSync(authDir, { recursive: true });

  const { state, saveCreds } = await useMultiFileAuthState(authDir);
  const { version } = await fetchLatestBaileysVersion();

  sock = makeWASocket({
    auth: state,
    version,
    printQRInTerminal: false,
    browser: ["Omni", "Desktop", "1.0.0"],
    syncFullHistory: false,
    markOnlineOnConnect: false,
  });

  // Handle credential updates (save session)
  sock.ev.on("creds.update", saveCreds);

  // Handle connection updates (QR code, connected, disconnected)
  sock.ev.on("connection.update", async (update) => {
    const { connection, lastDisconnect, qr } = update;

    if (qr) {
      // Generate QR code as base64 PNG
      try {
        const qrImage = await QRCode.toDataURL(qr, { width: 256 });
        emitEvent("qr", { qr, qrImage });
      } catch (err) {
        emitEvent("qr", { qr, qrImage: null, error: err.message });
      }
    }

    if (connection === "open") {
      reconnectAttempts = 0; // Reset on successful connection
      wasEverConnected = true;
      const user = sock.user;
      emitEvent("connected", {
        phone: user?.id?.split(":")[0] || "",
        name: user?.name || "",
      });
    }

    if (connection === "close") {
      const reason = lastDisconnect?.error?.output?.statusCode;
      const loggedOut = reason === DisconnectReason.loggedOut;

      sock = null;

      if (loggedOut && authDir && reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
        // Stale auth rejected — clear and auto-reconnect for fresh QR
        reconnectAttempts++;
        try {
          fs.rmSync(authDir, { recursive: true, force: true });
        } catch {}
        emitEvent("reconnecting", { reason: "logged_out" });
        try {
          await handleConnect(null, { authDir });
        } catch (err) {
          emitEvent("error", { message: `Auto-recovery failed: ${err.message}` });
          emitEvent("disconnected", { reason: "recovery_failed", loggedOut: true });
        }
        return;
      }

      if (!wasEverConnected && authDir && reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
        // QR expired before user could scan — auto-reconnect for fresh QR
        reconnectAttempts++;
        emitEvent("reconnecting", { reason: "qr_expired" });
        try {
          await handleConnect(null, { authDir });
        } catch (err) {
          emitEvent("error", { message: `QR refresh failed: ${err.message}` });
          emitEvent("disconnected", { reason: "qr_refresh_failed", loggedOut: false });
        }
        return;
      }

      emitEvent("disconnected", {
        reason: reason || "unknown",
        loggedOut,
      });
    }
  });

  // Handle incoming messages
  sock.ev.on("messages.upsert", ({ messages, type }) => {
    for (const msg of messages) {
      // Skip status broadcasts and own messages
      if (msg.key.remoteJid === "status@broadcast") continue;
      if (msg.key.fromMe) continue;

      const text =
        msg.message?.conversation ||
        msg.message?.extendedTextMessage?.text ||
        "";

      if (!text) continue;

      const from = msg.key.remoteJid || "";
      const isGroup = from.endsWith("@g.us");
      const sender = isGroup
        ? msg.key.participant || from
        : from;

      // Normalize phone number (remove @s.whatsapp.net suffix)
      const phone = sender.split("@")[0];

      emitEvent("message", {
        id: msg.key.id || "",
        from: phone,
        text,
        isGroup,
        groupId: isGroup ? from.split("@")[0] : null,
        timestamp: msg.messageTimestamp
          ? Number(msg.messageTimestamp) * 1000
          : Date.now(),
        pushName: msg.pushName || null,
      });
    }
  });

  if (id !== null) {
    respond(id, "ok");
  }
}

async function handleSend(id, params) {
  if (!sock) {
    respondError(id, "Not connected");
    return;
  }

  const { to, text } = params;
  if (!to || !text) {
    respondError(id, "Missing 'to' or 'text' parameter");
    return;
  }

  // Normalize recipient to JID format
  let jid = to;
  if (!jid.includes("@")) {
    // Assume it's a phone number — add WhatsApp suffix
    jid = jid.replace(/[^0-9]/g, "") + "@s.whatsapp.net";
  }

  try {
    const result = await sock.sendMessage(jid, { text });
    respond(id, { messageId: result?.key?.id || "" });
  } catch (err) {
    respondError(id, `Send failed: ${err.message}`);
  }
}

async function handleDisconnect(id) {
  if (sock) {
    try {
      sock.ev.removeAllListeners();
      sock.end(undefined);
    } catch {
      // Ignore disconnect errors
    }
    sock = null;
  }
  if (id !== null) {
    respond(id, "ok");
  }
}

async function handleLogout(id) {
  // Logout: unlink device from WhatsApp + clear local auth state
  if (sock) {
    try {
      await sock.logout();
    } catch {
      // If logout() fails (e.g. already disconnected), just clean up locally
      try {
        sock.ev.removeAllListeners();
        sock.end(undefined);
      } catch {
        // Ignore
      }
    }
    sock = null;
  }

  // Delete auth directory to force fresh QR on next connect
  if (authDir) {
    try {
      fs.rmSync(authDir, { recursive: true, force: true });
    } catch (err) {
      emitEvent("error", { message: `Failed to clear auth: ${err.message}` });
    }
  }

  if (id !== null) {
    respond(id, "ok");
  }
}

// Set up stdin line reader
const rl = createInterface({
  input: process.stdin,
  terminal: false,
});

rl.on("line", handleCommand);

// Handle process exit gracefully
process.on("SIGINT", async () => {
  await handleDisconnect(null);
  process.exit(0);
});

process.on("SIGTERM", async () => {
  await handleDisconnect(null);
  process.exit(0);
});

// Prevent unhandled rejections from crashing the sidecar
process.on("unhandledRejection", (reason) => {
  emitEvent("error", { message: `Unhandled rejection: ${reason}` });
});

process.on("uncaughtException", (err) => {
  emitEvent("error", { message: `Uncaught exception: ${err.message}` });
});

// Signal ready
emitEvent("ready", { version: "0.1.0" });
