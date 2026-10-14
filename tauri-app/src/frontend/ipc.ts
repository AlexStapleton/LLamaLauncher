import type { Config, Profile, Report, Heartbeat, ModelInfo, PlatformInfo, LaunchProfile, ProfileSummary } from './native-types';
interface Commands {
  profiles_list: [undefined, ProfileSummary[]];
  profiles_load: [{id:string}, LaunchProfile];
  profiles_save: [{id:string|null;name:string;config:Config}, LaunchProfile];
  profiles_copy: [{id:string;name:string;rename:boolean}, LaunchProfile];
  profiles_delete: [{id:string}, void];
  get_defaults: [undefined, Config];
  platform_info: [undefined, PlatformInfo];
  list_models: [{directory: string; force?: boolean}, ModelInfo[]];
  cancel_models: [undefined, void];
  server_help: [{binary: string; force?: boolean}, string];
  command_preview: [{config: Config}, string[]];
  start_server: [{config: Config}, string];
  stop_server: [undefined, void];
  server_status: [undefined, string];
  server_logs: [undefined, string];
  browse_path: [{kind: string; directory: string | null}, string | null];
  save_config: [{config: Config}, string | null];
  load_config: [undefined, Config | null];
  mcp_connect: [{profile: Profile}, Report];
  mcp_test: [{profile: Profile}, Report];
  mcp_disconnect: [undefined, void];
  mcp_status: [undefined, Heartbeat];
}
declare global {
  interface Window {
    __TAURI_INVOKE__: (command: string, args?: unknown) => Promise<unknown>;
  }
}
export async function invoke<K extends keyof Commands>(command: K, ...args: Commands[K][0] extends undefined ? [] : [Commands[K][0]]): Promise<Commands[K][1]> {
  return await window.__TAURI_INVOKE__(command, args[0]) as Commands[K][1];
}
