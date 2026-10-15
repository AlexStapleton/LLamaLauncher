// Generated from Rust DTOs by npm run types:generate. Do not edit.
import type {JobResult} from "./job-results";
export interface Config {
  schema_version: number;
  runtime_id: string;
  model_id: string;
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
export interface MonitoringPreferences {
  enabled: boolean;
  interval_seconds: number;
}
export interface HardwareSnapshot {
  cpu: string;
  logical_cores: number;
  total_ram: number;
  available_ram: number;
  gpus: Record<string, unknown>[];
}
export interface TelemetrySample {
  timestamp: number;
  cpu_percent: number;
  server_cpu_percent: number | null;
  server_ram: number | null;
  available_ram: number;
  gpus: Record<string, unknown>[];
  metrics: Record<string, unknown>;
}
export interface ModelMetadata {
  path: string;
  architecture: string;
  name: string;
  parameters: number;
  bytes: number;
  context: number;
  quantization: string;
  metadata: Record<string, unknown>;
  warnings: string[];
  estimate_low: number;
  estimate_high: number;
  resource_estimate: Record<string, unknown>;
}
export interface JobReport {
  id: string;
  kind: string;
  phase: string;
  message: string;
  completed: number;
  total: number;
  result: JobResult;
}
export interface DownloadFile {
  name: string;
  size: number;
  sha256: string;
}
export interface DownloadPlan {
  id: string;
  repo: string;
  revision: string;
  root: string;
  files: DownloadFile[];
}
export interface Artifact {
  id: string;
  kind: string;
  name: string;
  path: string;
  version: string;
  managed: boolean;
  details: Record<string, unknown>;
}
