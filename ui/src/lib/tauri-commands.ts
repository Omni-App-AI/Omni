import { invoke } from "@tauri-apps/api/core";

// Types matching Rust DTOs
export interface SessionDto {
  id: string;
  created_at: string;
  updated_at: string;
  metadata: string | null;
}

export interface MessageDto {
  role: string;
  content: string;
  tool_calls: string | null;
}

export interface ExtensionDto {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  enabled: boolean;
  active: boolean;
  tools: string[];
  permissions: string[];
  instance_count: number;
}

export interface ExtensionInstanceDto {
  instance_id: string;
  extension_id: string;
  instance_name: string;
  display_name: string | null;
  enabled: boolean;
  active: boolean;
  tools: string[];
  permissions: string[];
}

export interface GuardianMetricsDto {
  scan_count: number;
  block_count: number;
  signature_blocks: number;
  heuristic_blocks: number;
  ml_blocks: number;
  policy_blocks: number;
  avg_scan_ms: number;
  total_scans_db: number;
  total_blocked_db: number;
}

export interface AuditEntryDto {
  timestamp: string;
  event_type: string;
  extension_id: string;
  capability: string;
  decision: string;
}

export interface PendingBlockDto {
  scan_id: string;
  scan_type: string;
  layer: string;
  reason: string;
  confidence: number;
  content_preview: string;
  created_at: string;
}

// Command wrappers
export const sendMessage = (sessionId: string, message: string) =>
  invoke<string>("send_message", { sessionId, message });

export const getSessionMessages = (sessionId: string) =>
  invoke<MessageDto[]>("get_session_messages", { sessionId });

export const listSessions = () => invoke<SessionDto[]>("list_sessions");

export const createSession = (metadata?: string) =>
  invoke<string>("create_session", { metadata: metadata ?? null });

export const permissionRespond = (
  promptId: string,
  decision: string,
  duration: string,
) => invoke<void>("permission_respond", { promptId, decision, duration });

export const permissionRevoke = (extensionId: string) =>
  invoke<void>("permission_revoke", { extensionId });

export const killSwitch = () => invoke<number>("kill_switch");

export const installExtension = (sourcePath: string) =>
  invoke<string>("install_extension", { sourcePath });

export const listExtensions = () =>
  invoke<ExtensionDto[]>("list_extensions");

export const activateExtension = (extensionId: string) =>
  invoke<void>("activate_extension", { extensionId });

export const deactivateExtension = (extensionId: string) =>
  invoke<void>("deactivate_extension", { extensionId });

export const uninstallExtension = (extensionId: string) =>
  invoke<void>("uninstall_extension", { extensionId });

export const toggleExtensionEnabled = (extensionId: string, enabled: boolean) =>
  invoke<void>("toggle_extension_enabled", { extensionId, enabled });

export const getAuditLog = (extensionId?: string, limit?: number) =>
  invoke<AuditEntryDto[]>("get_audit_log", {
    extensionId: extensionId ?? null,
    limit: limit ?? null,
  });

export const getGuardianMetrics = () =>
  invoke<GuardianMetricsDto>("get_guardian_metrics");

export const guardianOverride = (scanId: string) =>
  invoke<void>("guardian_override", { scanId });

export const updateSettings = (settings: Record<string, unknown>) =>
  invoke<void>("update_settings", {
    settingsJson: JSON.stringify(settings),
  });

export const getSettings = () => invoke<string>("get_settings");

export const getPendingBlocks = () =>
  invoke<PendingBlockDto[]>("get_pending_blocks");

// Channel status
export type ChannelStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "reconnecting"
  | "error";

// Channel types
export interface ChannelFeaturesDto {
  direct_messages: boolean;
  group_messages: boolean;
  media_attachments: boolean;
  reactions: boolean;
  read_receipts: boolean;
  typing_indicators: boolean;
  threads: boolean;
}

export interface ChannelDto {
  id: string;
  channel_type: string;
  instance_id: string;
  name: string;
  status: string;
  features: ChannelFeaturesDto;
}

export interface ChannelTypeDto {
  channel_type: string;
  name: string;
  features: ChannelFeaturesDto;
}

// Channel command wrappers
export const channelList = () => invoke<ChannelDto[]>("channel_list");

export const channelListTypes = () =>
  invoke<ChannelTypeDto[]>("channel_list_types");

export const channelCreateInstance = (
  channelType: string,
  instanceId: string,
  displayName?: string,
) =>
  invoke<string>("channel_create_instance", {
    channelType,
    instanceId,
    displayName: displayName ?? null,
  });

export const channelRemoveInstance = (
  channelType: string,
  instanceId: string,
) => invoke<void>("channel_remove_instance", { channelType, instanceId });

export const channelConnect = (
  channelId: string,
  settings: Record<string, unknown>,
) => invoke<void>("channel_connect", { channelId, settings });

export const channelDisconnect = (channelId: string) =>
  invoke<void>("channel_disconnect", { channelId });

export const channelLogin = (
  channelId: string,
  credentialType: string,
  data: Record<string, string>,
) => invoke<string>("channel_login", { channelId, credentialType, data });

export const channelSend = (
  channelId: string,
  recipient: string,
  text: string,
  mediaUrl?: string,
  replyTo?: string,
) =>
  invoke<void>("channel_send", {
    channelId,
    recipient,
    text,
    mediaUrl: mediaUrl ?? null,
    replyTo: replyTo ?? null,
  });

export const channelGetApiKey = (channelId: string) =>
  invoke<string | null>("channel_get_api_key", { channelId });

// Binding types
export interface BindingDto {
  id: string;
  channel_instance: string;
  extension_id: string;
  peer_filter: string | null;
  group_filter: string | null;
  priority: number;
  enabled: boolean;
}

// Binding command wrappers
export const bindingAdd = (
  channelInstance: string,
  extensionId: string,
  peerFilter?: string,
  groupFilter?: string,
  priority?: number,
) =>
  invoke<string>("binding_add", {
    channelInstance,
    extensionId,
    peerFilter: peerFilter ?? null,
    groupFilter: groupFilter ?? null,
    priority: priority ?? null,
  });

export const bindingRemove = (bindingId: string) =>
  invoke<void>("binding_remove", { bindingId });

export const bindingList = () => invoke<BindingDto[]>("binding_list");

export const bindingListForExtension = (extensionId: string) =>
  invoke<BindingDto[]>("binding_list_for_extension", { extensionId });

// ─── Extension Instance Commands ────────────────────────────────────

export const createExtensionInstance = (
  extensionId: string,
  instanceName: string,
  displayName?: string,
) =>
  invoke<string>("create_extension_instance", {
    extensionId,
    instanceName,
    displayName: displayName ?? null,
  });

export const deleteExtensionInstance = (instanceId: string) =>
  invoke<void>("delete_extension_instance", { instanceId });

export const listExtensionInstances = (extensionId?: string) =>
  invoke<ExtensionInstanceDto[]>("list_extension_instances", {
    extensionId: extensionId ?? null,
  });

export const updateExtensionInstance = (
  instanceId: string,
  displayName?: string,
) =>
  invoke<void>("update_extension_instance", {
    instanceId,
    displayName: displayName ?? null,
  });

export const activateExtensionInstance = (instanceId: string) =>
  invoke<void>("activate_extension_instance", { instanceId });

export const deactivateExtensionInstance = (instanceId: string) =>
  invoke<void>("deactivate_extension_instance", { instanceId });

export const toggleInstanceEnabled = (instanceId: string, enabled: boolean) =>
  invoke<void>("toggle_instance_enabled", { instanceId, enabled });

// ─── Provider Types ─────────────────────────────────────────────────

export interface ProviderDto {
  id: string;
  provider_type: string;
  display_name: string;
  default_model: string | null;
  endpoint: string | null;
  max_tokens: number | null;
  temperature: number | null;
  enabled: boolean;
  has_credential: boolean;
  auth_type: string;
  env_var_hint: string | null;
}

export interface ProviderTypeInfoDto {
  provider_type: string;
  display_name: string;
  auth_type: string;
  env_var_hint: string | null;
  default_endpoint: string | null;
  description: string;
}

// ─── Provider Command Wrappers ──────────────────────────────────────

export const providerListTypes = () =>
  invoke<ProviderTypeInfoDto[]>("provider_list_types");

export const providerList = () => invoke<ProviderDto[]>("provider_list");

export const providerAdd = (
  id: string,
  providerType: string,
  defaultModel?: string,
  endpoint?: string,
  maxTokens?: number,
  temperature?: number,
) =>
  invoke<void>("provider_add", {
    id,
    providerType,
    defaultModel: defaultModel ?? null,
    endpoint: endpoint ?? null,
    maxTokens: maxTokens ?? null,
    temperature: temperature ?? null,
  });

export const providerUpdate = (
  id: string,
  defaultModel?: string,
  endpoint?: string,
  maxTokens?: number,
  temperature?: number,
  enabled?: boolean,
) =>
  invoke<void>("provider_update", {
    id,
    defaultModel: defaultModel ?? null,
    endpoint: endpoint ?? null,
    maxTokens: maxTokens ?? null,
    temperature: temperature ?? null,
    enabled: enabled ?? null,
  });

export const providerRemove = (id: string) =>
  invoke<void>("provider_remove", { id });

export const providerSetCredential = (
  providerId: string,
  credentialType: string,
  apiKey?: string,
  awsAccessKeyId?: string,
  awsSecretAccessKey?: string,
  awsSessionToken?: string,
  awsRegion?: string,
) =>
  invoke<void>("provider_set_credential", {
    providerId,
    credentialType,
    apiKey: apiKey ?? null,
    awsAccessKeyId: awsAccessKeyId ?? null,
    awsSecretAccessKey: awsSecretAccessKey ?? null,
    awsSessionToken: awsSessionToken ?? null,
    awsRegion: awsRegion ?? null,
  });

export const providerDeleteCredential = (providerId: string) =>
  invoke<void>("provider_delete_credential", { providerId });

export const providerTestCredential = (providerId: string) =>
  invoke<string>("provider_test_credential", { providerId });

// ─── Marketplace Types ──────────────────────────────────────────

export interface MarketplaceExtensionDto {
  id: string;
  name: string;
  short_description: string;
  icon_url: string | null;
  categories: string[];
  tags: string[];
  trust_level: string;
  latest_version: string;
  total_downloads: number;
  average_rating: number;
  review_count: number;
  publisher_name: string;
  publisher_verified: boolean;
}

export interface MarketplaceDetailDto {
  id: string;
  name: string;
  short_description: string;
  description: string;
  icon_url: string | null;
  categories: string[];
  tags: string[];
  trust_level: string;
  latest_version: string;
  total_downloads: number;
  average_rating: number;
  review_count: number;
  publisher_name: string;
  publisher_verified: boolean;
  homepage: string | null;
  repository: string | null;
  license: string | null;
  min_omni_version: string | null;
  tools: string[];
  permissions: string[];
  changelog: string | null;
  screenshots: string[];
  scan_status: string | null;
  scan_score: number | null;
  wasm_size_bytes: number | null;
}

export interface MarketplaceSearchResultDto {
  extensions: MarketplaceExtensionDto[];
  total: number;
  page: number;
  limit: number;
  total_pages: number;
}

export interface MarketplaceCategoryDto {
  id: string;
  name: string;
  icon: string;
  count: number;
}

export interface ExtensionUpdateDto {
  extension_id: string;
  installed_version: string;
  latest_version: string;
  has_update: boolean;
}

// ─── Marketplace Commands ───────────────────────────────────────

export const marketplaceSearch = (
  query?: string,
  category?: string,
  sort?: string,
  trust?: string,
  page?: number,
  limit?: number,
  forceRefresh?: boolean,
) =>
  invoke<MarketplaceSearchResultDto>("marketplace_search", {
    query: query ?? null,
    category: category ?? null,
    sort: sort ?? null,
    trust: trust ?? null,
    page: page ?? null,
    limit: limit ?? null,
    forceRefresh: forceRefresh ?? null,
  });

export const marketplaceGetDetail = (extensionId: string, forceRefresh?: boolean) =>
  invoke<MarketplaceDetailDto>("marketplace_get_detail", {
    extensionId,
    forceRefresh: forceRefresh ?? null,
  });

export const marketplaceGetCategories = (forceRefresh?: boolean) =>
  invoke<MarketplaceCategoryDto[]>("marketplace_get_categories", {
    forceRefresh: forceRefresh ?? null,
  });

export const marketplaceInstall = (extensionId: string) =>
  invoke<string>("marketplace_install", { extensionId });

export const marketplaceCheckUpdates = () =>
  invoke<ExtensionUpdateDto[]>("marketplace_check_updates");

// ─── MCP Types ──────────────────────────────────────────────────

export interface McpToolDto {
  name: string;
  description: string | null;
}

export interface McpServerDto {
  name: string;
  status: string;
  tool_count: number;
  tools: McpToolDto[];
  command: string;
  args: string[];
  env: Record<string, string>;
  working_dir: string | null;
  auto_start: boolean;
}

// ─── MCP Commands ───────────────────────────────────────────────

export const mcpListServers = () =>
  invoke<McpServerDto[]>("mcp_list_servers");

export const mcpAddServer = (
  name: string,
  command: string,
  args: string[],
  env: Record<string, string>,
  workingDir?: string,
  autoStart?: boolean,
  connectNow?: boolean,
) =>
  invoke<void>("mcp_add_server", {
    name,
    command,
    args,
    env,
    workingDir: workingDir ?? null,
    autoStart: autoStart ?? true,
    connectNow: connectNow ?? true,
  });

export const mcpRemoveServer = (name: string) =>
  invoke<void>("mcp_remove_server", { name });

export const mcpUpdateServer = (
  name: string,
  command?: string,
  args?: string[],
  env?: Record<string, string>,
  workingDir?: string,
  autoStart?: boolean,
) =>
  invoke<void>("mcp_update_server", {
    name,
    command: command ?? null,
    args: args ?? null,
    env: env ?? null,
    workingDir: workingDir ?? null,
    autoStart: autoStart ?? null,
  });

export const mcpStartServer = (name: string) =>
  invoke<void>("mcp_start_server", { name });

export const mcpStopServer = (name: string) =>
  invoke<void>("mcp_stop_server", { name });

export const mcpRestartServer = (name: string) =>
  invoke<void>("mcp_restart_server", { name });

export const mcpGetServerTools = (name: string) =>
  invoke<McpToolDto[]>("mcp_get_server_tools", { name });

// ─── Flowchart Types ────────────────────────────────────────────

export interface FlowchartDto {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  enabled: boolean;
  tool_count: number;
  permission_count: number;
}

export interface FlowchartToolDefDto {
  name: string;
  description: string;
  parameters: unknown;
  trigger_node_id: string;
}

export interface FlowchartPermissionDto {
  capability: string;
  reason: string;
  required: boolean;
}

export interface FlowchartDefinitionDto {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  enabled: boolean;
  tools: FlowchartToolDefDto[];
  permissions: FlowchartPermissionDto[];
  config: Record<string, unknown>;
  nodes: unknown[];
  edges: unknown[];
  viewport: { x: number; y: number; zoom: number } | null;
  created_at: string;
  updated_at: string;
}

export interface FlowchartNodeTraceDto {
  node_id: string;
  node_type: string;
  label: string;
  duration_ms: number;
  error: string | null;
  input?: unknown;
  output?: unknown;
}

export interface FlowchartTestResultDto {
  success: boolean;
  output: unknown | null;
  error: string | null;
  execution_time_ms: number;
  node_trace: FlowchartNodeTraceDto[];
}

export interface FlowchartValidationDto {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

// ─── Flowchart Commands ─────────────────────────────────────────

export const flowchartList = () =>
  invoke<FlowchartDto[]>("flowchart_list");

export const flowchartGet = (flowchartId: string) =>
  invoke<FlowchartDefinitionDto>("flowchart_get", { flowchartId });

export const flowchartSave = (definition: FlowchartDefinitionDto) =>
  invoke<void>("flowchart_save", { definition });

export const flowchartDelete = (flowchartId: string) =>
  invoke<void>("flowchart_delete", { flowchartId });

export const flowchartToggleEnabled = (flowchartId: string, enabled: boolean) =>
  invoke<void>("flowchart_toggle_enabled", { flowchartId, enabled });

export const flowchartValidate = (definition: FlowchartDefinitionDto) =>
  invoke<FlowchartValidationDto>("flowchart_validate", { definition });

export const flowchartTest = (flowchartId: string, toolName: string, testParams: Record<string, unknown>) =>
  invoke<FlowchartTestResultDto>("flowchart_test", { flowchartId, toolName, testParams });

// ─── Environment Variable Commands ──────────────────────────────────

export interface EnvVarEntryDto {
  key: string;
  masked_value: string;
  is_set: boolean;
}

export const envVarsList = () =>
  invoke<EnvVarEntryDto[]>("env_vars_list");

export const envVarsSet = (key: string, value: string) =>
  invoke<void>("env_vars_set", { key, value });

export const envVarsDelete = (key: string) =>
  invoke<void>("env_vars_delete", { key });
