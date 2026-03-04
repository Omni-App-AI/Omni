export interface CredentialField {
  key: string;
  label: string;
  type: "text" | "password" | "textarea";
  placeholder?: string;
  required: boolean;
  helpText?: string;
}

export type ChannelCategory =
  | "popular"
  | "team"
  | "social"
  | "self-hosted"
  | "decentralized";

export type AuthMode = "credentials" | "qr_code" | "none";

export interface ChannelMeta {
  displayName: string;
  description: string;
  category: ChannelCategory;
  icon: string;
  authMode: AuthMode;
  credentialType: string;
  fields: CredentialField[];
}

export const CHANNEL_CATEGORIES: {
  key: ChannelCategory;
  label: string;
}[] = [
  { key: "popular", label: "Popular" },
  { key: "team", label: "Team Chat" },
  { key: "social", label: "Social" },
  { key: "self-hosted", label: "Self-Hosted" },
  { key: "decentralized", label: "Decentralized" },
];

export const CHANNEL_META: Record<string, ChannelMeta> = {
  discord: {
    displayName: "Discord",
    description: "Connect a Discord bot to your server",
    category: "popular",
    icon: "MessageCircle",
    authMode: "credentials",
    credentialType: "bot_token",
    fields: [
      {
        key: "token",
        label: "Bot Token",
        type: "password",
        placeholder: "Paste your Discord bot token",
        required: true,
        helpText: "Create a bot at discord.com/developers/applications",
      },
    ],
  },
  telegram: {
    displayName: "Telegram",
    description: "Connect a Telegram bot via BotFather",
    category: "popular",
    icon: "Send",
    authMode: "credentials",
    credentialType: "bot_token",
    fields: [
      {
        key: "token",
        label: "Bot Token",
        type: "password",
        placeholder: "Paste your Telegram bot token",
        required: true,
        helpText: "Get your token from @BotFather on Telegram",
      },
    ],
  },
  "whatsapp-web": {
    displayName: "WhatsApp",
    description: "Connect via WhatsApp Web QR code scan",
    category: "popular",
    icon: "Phone",
    authMode: "qr_code",
    credentialType: "",
    fields: [],
  },
  slack: {
    displayName: "Slack",
    description: "Connect a Slack bot to your workspace",
    category: "popular",
    icon: "Hash",
    authMode: "credentials",
    credentialType: "bot_token",
    fields: [
      {
        key: "bot_token",
        label: "Bot Token",
        type: "password",
        placeholder: "xoxb-...",
        required: true,
        helpText: "Found in your Slack app's OAuth & Permissions page",
      },
      {
        key: "app_token",
        label: "App-Level Token",
        type: "password",
        placeholder: "xapp-...",
        required: false,
        helpText: "Required for Socket Mode (recommended)",
      },
    ],
  },
  mattermost: {
    displayName: "Mattermost",
    description: "Connect to a Mattermost server",
    category: "team",
    icon: "MessageSquare",
    authMode: "credentials",
    credentialType: "api_key",
    fields: [
      {
        key: "server_url",
        label: "Server URL",
        type: "text",
        placeholder: "https://mattermost.example.com",
        required: true,
      },
      {
        key: "token",
        label: "Personal Access Token",
        type: "password",
        placeholder: "Paste your access token",
        required: true,
        helpText: "Generate at Account Settings > Security > Personal Access Tokens",
      },
    ],
  },
  line: {
    displayName: "LINE",
    description: "Connect a LINE Messaging API channel",
    category: "team",
    icon: "MessageCircle",
    authMode: "credentials",
    credentialType: "api_key",
    fields: [
      {
        key: "channel_access_token",
        label: "Channel Access Token",
        type: "password",
        placeholder: "Paste your channel access token",
        required: true,
      },
      {
        key: "channel_secret",
        label: "Channel Secret",
        type: "password",
        placeholder: "Paste your channel secret",
        required: true,
        helpText: "Found in your LINE Developers console",
      },
    ],
  },
  teams: {
    displayName: "Microsoft Teams",
    description: "Connect a Teams bot via Azure Bot Service",
    category: "team",
    icon: "Users",
    authMode: "credentials",
    credentialType: "oauth",
    fields: [
      {
        key: "app_id",
        label: "App ID",
        type: "text",
        placeholder: "Azure AD application ID",
        required: true,
      },
      {
        key: "app_password",
        label: "App Password",
        type: "password",
        placeholder: "Azure AD client secret",
        required: true,
        helpText: "From Azure Portal > App registrations > Certificates & secrets",
      },
    ],
  },
  "google-chat": {
    displayName: "Google Chat",
    description: "Connect via Google Workspace service account",
    category: "team",
    icon: "MessageSquare",
    authMode: "credentials",
    credentialType: "service_account",
    fields: [
      {
        key: "service_account_json",
        label: "Service Account JSON",
        type: "textarea",
        placeholder: '{"type": "service_account", ...}',
        required: true,
        helpText: "Download from Google Cloud Console > IAM & Admin > Service Accounts",
      },
    ],
  },
  feishu: {
    displayName: "Feishu / Lark",
    description: "Connect a Feishu (Lark) app",
    category: "team",
    icon: "MessageSquare",
    authMode: "credentials",
    credentialType: "app_credentials",
    fields: [
      {
        key: "app_id",
        label: "App ID",
        type: "text",
        placeholder: "cli_...",
        required: true,
      },
      {
        key: "app_secret",
        label: "App Secret",
        type: "password",
        placeholder: "Paste your app secret",
        required: true,
        helpText: "Found in Feishu Open Platform > App Credentials",
      },
    ],
  },
  irc: {
    displayName: "IRC",
    description: "Connect to an IRC server",
    category: "self-hosted",
    icon: "Terminal",
    authMode: "credentials",
    credentialType: "password",
    fields: [
      {
        key: "nickname",
        label: "Nickname",
        type: "text",
        placeholder: "omni-bot",
        required: true,
      },
      {
        key: "server",
        label: "Server",
        type: "text",
        placeholder: "irc.libera.chat",
        required: true,
      },
      {
        key: "port",
        label: "Port",
        type: "text",
        placeholder: "6667",
        required: false,
      },
      {
        key: "password",
        label: "Password",
        type: "password",
        placeholder: "Server password (if required)",
        required: false,
      },
    ],
  },
  twitch: {
    displayName: "Twitch",
    description: "Connect a Twitch chat bot",
    category: "social",
    icon: "Tv",
    authMode: "credentials",
    credentialType: "oauth",
    fields: [
      {
        key: "oauth_token",
        label: "OAuth Token",
        type: "password",
        placeholder: "oauth:...",
        required: true,
        helpText: "Generate at twitchapps.com/tmi or dev.twitch.tv",
      },
      {
        key: "username",
        label: "Bot Username",
        type: "text",
        placeholder: "Your bot's Twitch username",
        required: true,
      },
    ],
  },
  matrix: {
    displayName: "Matrix",
    description: "Connect to a Matrix homeserver",
    category: "self-hosted",
    icon: "Globe",
    authMode: "credentials",
    credentialType: "password",
    fields: [
      {
        key: "homeserver_url",
        label: "Homeserver URL",
        type: "text",
        placeholder: "https://matrix.org",
        required: true,
      },
      {
        key: "username",
        label: "Username",
        type: "text",
        placeholder: "@bot:matrix.org",
        required: true,
      },
      {
        key: "password",
        label: "Password",
        type: "password",
        placeholder: "Account password",
        required: true,
      },
    ],
  },
  nostr: {
    displayName: "Nostr",
    description: "Connect to the Nostr network",
    category: "decentralized",
    icon: "Zap",
    authMode: "credentials",
    credentialType: "private_key",
    fields: [
      {
        key: "private_key",
        label: "Private Key (hex)",
        type: "password",
        placeholder: "nsec or hex private key",
        required: true,
      },
      {
        key: "relays",
        label: "Relays",
        type: "text",
        placeholder: "wss://relay.damus.io, wss://nos.lol",
        required: false,
        helpText: "Comma-separated list of relay URLs",
      },
    ],
  },
  "nextcloud-talk": {
    displayName: "Nextcloud Talk",
    description: "Connect to Nextcloud Talk",
    category: "self-hosted",
    icon: "Cloud",
    authMode: "credentials",
    credentialType: "password",
    fields: [
      {
        key: "server_url",
        label: "Server URL",
        type: "text",
        placeholder: "https://cloud.example.com",
        required: true,
      },
      {
        key: "username",
        label: "Username",
        type: "text",
        placeholder: "bot-user",
        required: true,
      },
      {
        key: "app_password",
        label: "App Password",
        type: "password",
        placeholder: "Generated app password",
        required: true,
        helpText: "Generate at Settings > Security > Devices & sessions",
      },
    ],
  },
  "synology-chat": {
    displayName: "Synology Chat",
    description: "Connect to Synology Chat via webhooks",
    category: "self-hosted",
    icon: "Server",
    authMode: "credentials",
    credentialType: "webhook",
    fields: [
      {
        key: "outgoing_url",
        label: "Outgoing Webhook URL",
        type: "text",
        placeholder: "https://nas.example.com/webapi/...",
        required: true,
      },
      {
        key: "incoming_token",
        label: "Incoming Webhook Token",
        type: "password",
        placeholder: "Paste your webhook token",
        required: true,
      },
    ],
  },
  zalo: {
    displayName: "Zalo",
    description: "Connect a Zalo Official Account",
    category: "social",
    icon: "MessageCircle",
    authMode: "credentials",
    credentialType: "api_key",
    fields: [
      {
        key: "access_token",
        label: "OA Access Token",
        type: "password",
        placeholder: "Paste your OA access token",
        required: true,
      },
      {
        key: "oa_secret_key",
        label: "OA Secret Key",
        type: "password",
        placeholder: "Paste your OA secret key",
        required: false,
      },
    ],
  },
  bluebubbles: {
    displayName: "BlueBubbles",
    description: "Connect via BlueBubbles server (macOS)",
    category: "self-hosted",
    icon: "Smartphone",
    authMode: "credentials",
    credentialType: "api_key",
    fields: [
      {
        key: "server_url",
        label: "Server URL",
        type: "text",
        placeholder: "http://localhost:1234",
        required: true,
      },
      {
        key: "password",
        label: "Password",
        type: "password",
        placeholder: "BlueBubbles server password",
        required: true,
      },
    ],
  },
  imessage: {
    displayName: "iMessage",
    description: "Connect iMessage via BlueBubbles",
    category: "self-hosted",
    icon: "Smartphone",
    authMode: "credentials",
    credentialType: "api_key",
    fields: [
      {
        key: "server_url",
        label: "BlueBubbles Server URL",
        type: "text",
        placeholder: "http://localhost:1234",
        required: true,
        helpText: "Requires BlueBubbles running on a Mac",
      },
      {
        key: "password",
        label: "Password",
        type: "password",
        placeholder: "BlueBubbles server password",
        required: true,
      },
    ],
  },
  signal: {
    displayName: "Signal",
    description: "Connect Signal via QR code pairing",
    category: "self-hosted",
    icon: "Shield",
    authMode: "qr_code",
    credentialType: "",
    fields: [
      {
        key: "api_url",
        label: "Signal CLI REST API URL",
        type: "text",
        placeholder: "http://localhost:8080",
        required: false,
        helpText: "URL of your signal-cli-rest-api instance",
      },
      {
        key: "phone_number",
        label: "Phone Number",
        type: "text",
        placeholder: "+1234567890",
        required: false,
      },
    ],
  },
  urbit: {
    displayName: "Urbit",
    description: "Connect to an Urbit ship",
    category: "decentralized",
    icon: "Globe",
    authMode: "credentials",
    credentialType: "access_code",
    fields: [
      {
        key: "ship_url",
        label: "Ship URL",
        type: "text",
        placeholder: "http://localhost:8080",
        required: true,
      },
      {
        key: "ship_name",
        label: "Ship Name",
        type: "text",
        placeholder: "~zod",
        required: true,
      },
      {
        key: "access_code",
        label: "Access Code (+code)",
        type: "password",
        placeholder: "Paste your +code",
        required: true,
      },
    ],
  },
  webchat: {
    displayName: "WebChat",
    description: "Browser-based chat widget served on the webhook server",
    category: "self-hosted",
    icon: "Globe",
    authMode: "credentials",
    credentialType: "api_key",
    fields: [
      {
        key: "api_key",
        label: "API Key",
        type: "password",
        placeholder: "Leave empty to auto-generate a secure key",
        required: false,
        helpText: "Clients must send this in the x-api-key header",
      },
    ],
  },
  twitter: {
    displayName: "Twitter / X",
    description: "Connect via X API with OAuth 1.0a",
    category: "social",
    icon: "AtSign",
    authMode: "credentials",
    credentialType: "oauth1",
    fields: [
      {
        key: "api_key",
        label: "API Key",
        type: "password",
        placeholder: "Your X API key",
        required: true,
        helpText: "From developer.x.com > Projects & Apps",
      },
      {
        key: "api_secret",
        label: "API Secret",
        type: "password",
        placeholder: "Your X API secret",
        required: true,
      },
      {
        key: "access_token",
        label: "Access Token",
        type: "password",
        placeholder: "Your access token",
        required: true,
      },
      {
        key: "access_token_secret",
        label: "Access Token Secret",
        type: "password",
        placeholder: "Your access token secret",
        required: true,
      },
    ],
  },
};

/** Get metadata for a channel type, with a sensible fallback for unknown types. */
export function getChannelMeta(channelType: string): ChannelMeta {
  return (
    CHANNEL_META[channelType] ?? {
      displayName: channelType,
      description: `Connect to ${channelType}`,
      category: "self-hosted" as ChannelCategory,
      icon: "Radio",
      authMode: "credentials" as AuthMode,
      credentialType: "api_key",
      fields: [
        {
          key: "token",
          label: "API Token",
          type: "password" as const,
          placeholder: "Paste your API token",
          required: true,
        },
      ],
    }
  );
}
