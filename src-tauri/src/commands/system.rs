//! Системные метрики (sysinfo 0.30): CPU, RAM, диск, сеть, ядра.
use serde::Serialize;
use sysinfo::{Disks, Networks, System};

#[derive(Serialize, Clone)]
pub struct Metrics {
    pub cpu_percent: f32,
    pub cpu_cores: Vec<f32>,
    pub cpu_count: usize,
    pub ram_used_gb: f64,
    pub ram_total_gb: f64,
    pub ram_percent: f32,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
    pub disk_percent: f32,
    pub net_rx_mb: f64,
    pub net_tx_mb: f64,
    pub hostname: String,
    pub kernel: String,
}

#[tauri::command]
pub fn get_metrics() -> Metrics {
    let mut sys = System::new_all();
    sys.refresh_all();
    let cpu_percent = sys.global_cpu_info().cpu_usage();
    let cores: Vec<f32> = sys.cpus().iter().map(|c| c.cpu_usage()).collect();
    let cpu_count = cores.len();
    let total_mem = sys.total_memory() as f64 / 1e9;
    let used_mem = sys.used_memory() as f64 / 1e9;
    let ram_percent = if total_mem > 0.0 { (used_mem / total_mem * 100.0) as f32 } else { 0.0 };
    // Диск: Disks (0.30 API, не System::disks)
    let disks = Disks::new_with_refreshed_list();
    let disk = disks.iter().next().map(|d| {
        let total = d.total_space() as f64 / 1e9;
        let avail = d.available_space() as f64 / 1e9;
        let used = total - avail;
        (used, total, if total > 0.0 { (used / total * 100.0) as f32 } else { 0.0 })
    }).unwrap_or((0.0, 0.0, 0.0));
    // Сеть: Networks (0.30 API)
    let networks = Networks::new_with_refreshed_list();
    let net = networks.values().next().map(|n| {
        (n.received() as f64 / 1e6, n.transmitted() as f64 / 1e6)
    }).unwrap_or((0.0, 0.0));
    Metrics {
        cpu_percent,
        cpu_cores: cores,
        cpu_count,
        ram_used_gb: used_mem,
        ram_total_gb: total_mem,
        ram_percent,
        disk_used_gb: disk.0,
        disk_total_gb: disk.1,
        disk_percent: disk.2,
        net_rx_mb: net.0,
        net_tx_mb: net.1,
        hostname: System::host_name().unwrap_or_else(|| "localhost".into()),
        kernel: System::kernel_version().unwrap_or_else(|| "unknown".into()),
    }
}
