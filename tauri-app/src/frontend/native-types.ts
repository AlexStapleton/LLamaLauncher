// Generated from Rust DTOs by npm run types:generate. Do not edit.
export interface Config {
  schema_version: number;
  llama_cpp_path: string;
  models_path: string;
  model_file: string;
  host: string;
  port: number;
  argument_overrides: ArgumentOverride[];
  additional_args: string[];
  router_mode: boolean;
  mcp_profile: Profile;
}
export interface ArgumentOverride {
  flag: string;
  values: string[];
}
export interface ModelInfo {
  name: string;
  path: string;
  size: number;
}
export interface PlatformInfo {
  windows: boolean;
}
export interface Profile {
  name: string;
  transport: string;
  url: string;
  bearer_token: string;
  command: string;
  args: string[];
  env: Record<string, string>;
  cwd: string;
  enabled: boolean;
}
export interface Report {
  connected: boolean;
  server: string;
  endpoint: string;
  protocol: string;
  tools: Record<string, unknown>[];
  message: string;
}
export interface Heartbeat {
  connected: boolean;
  message: string;
}
export interface LaunchProfile {
  id: string;
  name: string;
  config: Config;
}
export interface ProfileSummary {
  id: string;
  name: string;
  runtime: string;
  model: string;
  router: boolean;
  overrides: number;
}
