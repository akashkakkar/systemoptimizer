import { invoke } from "@tauri-apps/api/core";

export interface DiskUsageReport {
  mount_point: string;
  fs_type: string;
  total_bytes: number;
  used_bytes: number;
  available_bytes: number;
  usage_percent: number;
}

export async function getDiskUsage(): Promise<DiskUsageReport[]> {
  return invoke<DiskUsageReport[]>("get_disk_usage");
}

export async function getAppVersion(): Promise<string> {
  return invoke<string>("get_app_version");
}
