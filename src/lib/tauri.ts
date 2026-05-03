import { invoke } from "@tauri-apps/api/core";

export interface DiskUsageReport {
  mount_point: string;
  fs_type: string;
  total_bytes: number;
  used_bytes: number;
  available_bytes: number;
  usage_percent: number;
}

export interface MemoryInfo {
  total_bytes: number;
  used_bytes: number;
  available_bytes: number;
  swap_total_bytes: number;
  swap_used_bytes: number;
  usage_percent: number;
  swap_percent: number;
}

export interface CpuInfo {
  core_count: number;
  per_core_percent: number[];
  load_avg_1: number;
  load_avg_5: number;
  load_avg_15: number;
}

export interface ProcessInfo {
  pid: number;
  name: string;
  cpu_percent: number;
  memory_percent: number;
  status: string;
}

export async function getDiskUsage(): Promise<DiskUsageReport[]> {
  return invoke<DiskUsageReport[]>("get_disk_usage");
}

export async function getAppVersion(): Promise<string> {
  return invoke<string>("get_app_version");
}

export async function getMemoryUsage(): Promise<MemoryInfo> {
  return invoke<MemoryInfo>("get_memory_usage");
}

export async function getCpuUsage(): Promise<CpuInfo> {
  return invoke<CpuInfo>("get_cpu_usage");
}

export async function getProcessList(): Promise<ProcessInfo[]> {
  return invoke<ProcessInfo[]>("get_process_list");
}
