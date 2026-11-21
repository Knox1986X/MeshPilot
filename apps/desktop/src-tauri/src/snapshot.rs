//! Реальные локальные метрики: чтение /proc, /sys, statvfs(2) и только они. Основная платформа MeshPilot — Linux (CachyOS/Arch).
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;

use crate::util;

// ---------- DTO (camelCase для TS) ----------

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub fs_type: Option<String>,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub deduped_from: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub cpu_total_percent: Option<f32>,
    pub cpu_per_core: Vec<Option<f32>>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    pub swap_used_bytes: Option<u64>,
    pub swap_total_bytes: Option<u64>,
    pub disk_used_bytes: Option<u64>,
    pub disk_total_bytes: Option<u64>,
    pub disks: Vec<DiskInfo>,
    pub network_rx_bytes_per_sec: Option<f64>,
    pub network_tx_bytes_per_sec: Option<f64>,
    pub temperature_celsius: Option<f32>,
    pub uptime_seconds: Option<u64>,
    pub hostname: Option<String>,
    pub kernel: Option<String>,
    pub os_name: Option<String>,
    pub desktop: Option<String>,
    pub updated_at_utc: String,
    pub partial: Vec<String>,
}

// ---------- Чистые парсеры (тестируемы) ----------

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CpuTimes {
    pub idle_all: u64,
    pub total: u64,
}

/// Разбирает /proc/stat: (строка "cpu" → CpuTimes, cpu0..cpuN → Vec<CpuTimes>).
pub fn parse_proc_stat(raw: &str) -> (Option<CpuTimes>, Vec<CpuTimes>) {
    let mut global = None;
    let mut cores = Vec::new();
    for line in raw.lines() {
        let mut parts = line.split_whitespace();
        let Some(label) = parts.next() else { continue };
        if !label.starts_with("cpu") {
            continue;
        }
        let nums: Vec<u64> = parts.filter_map(|p| p.parse().ok()).collect();
        if nums.len() < 5 {
            continue;
        }
        // user nice system idle iowait irq softirq steal (guest* не учитываем — уже включены в user)
        let take = nums.len().min(8);
        let slice = &nums[..take];
        let idle_all = slice.get(3).copied().unwrap_or(0) + slice.get(4).copied().unwrap_or(0);
        let total: u64 = slice.iter().sum();
        let ct = CpuTimes { idle_all, total };
        if label == "cpu" {
            global = Some(ct);
        } else if label[3..].chars().all(|c| c.is_ascii_digit()) {
            cores.push(ct);
        }
    }
    (global, cores)
}

/// Разбирает /proc/meminfo → словарь «ключ → kB».
pub fn parse_meminfo(raw: &str) -> HashMap<String, u64> {
    let mut out = HashMap::new();
    for line in raw.lines() {
        let mut parts = line.split_whitespace();
        let Some(key) = parts.next() else { continue };
        let key = key.trim_end_matches(':');
        let Some(value) = parts.next() else { continue };
        if let Ok(kb) = value.parse::<u64>() {
            out.insert(key.to_string(), kb);
        }
    }
    out
}

/// Разбирает /proc/net/dev → «iface → (rx_bytes, tx_bytes)», кроме loopback.
pub fn parse_net_dev(raw: &str) -> Vec<(String, u64, u64)> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let Some((iface, rest)) = line.split_once(':') else { continue };
        let iface = iface.trim().to_string();
        if iface == "lo" || iface.is_empty() {
            continue;
        }
        let nums: Vec<u64> = rest.split_whitespace().filter_map(|p| p.parse().ok()).collect();
        if nums.len() >= 10 {
            out.push((iface, nums[0], nums[8]));
        }
    }
    out
}

const ALLOWED_FS: &[&str] = &[
    "btrfs", "ext2", "ext3", "ext4", "xfs", "f2fs", "zfs", "bcachefs", "reiserfs", "jfs",
    "ntfs", "ntfs3", "exfat", "vfat", "hfsplus", "apfs", "udf",
];

pub struct MountEntry {
    pub device: String,
    pub mount_point: String,
    pub fs_type: String,
}

/// Разбирает /proc/mounts, оставляя только «настоящие» файловые системы.
pub fn parse_mounts(raw: &str) -> Vec<MountEntry> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 3 {
            continue;
        }
        let (device, mount_point, fs_type) = (fields[0], fields[1], fields[2]);
        if !ALLOWED_FS.contains(&fs_type) {
            continue;
        }
        if mount_point.starts_with("/proc") || mount_point.starts_with("/sys") || mount_point.starts_with("/dev") {
            continue;
        }
        // декодируем \040 (пробелы) в пути
        let mount_point = mount_point.replace("\\040", " ");
        out.push(MountEntry {
            device: device.to_string(),
            mount_point,
            fs_type: fs_type.to_string(),
        });
    }
    out
}

/// Дедупликация точек монтирования одной ФС: ключ — устройство, из группы
/// сохраняется самый короткий путь (ближе к корню), остальные идут в dedupedFrom.
pub fn dedupe_mounts(entries: Vec<MountEntry>, sizes: &HashMap<String, (u64, u64, u64)>) -> Vec<DiskInfo> {
    let mut by_device: HashMap<String, (MountEntry, u64, u64, u64, Vec<String>)> = HashMap::new();
    for m in entries {
        let Some((total, used, avail)) = sizes.get(&m.mount_point).copied() else { continue };
        by_device
            .entry(m.device.clone())
            .and_modify(|(keep, _, _, _, dup)| {
                if m.mount_point.len() < keep.mount_point.len() {
                    dup.push(std::mem::replace(&mut keep.mount_point, m.mount_point.clone()));
                    keep.fs_type = m.fs_type.clone();
                } else {
                    dup.push(m.mount_point.clone());
                }
            })
            .or_insert((m, total, used, avail, Vec::new()));
    }
    let mut out: Vec<DiskInfo> = by_device
        .into_values()
        .map(|(keep, total, used, avail, mut dup)| {
            dup.sort();
            DiskInfo {
                name: keep.device,
                mount_point: keep.mount_point,
                fs_type: Some(keep.fs_type),
                total_bytes: total,
                used_bytes: used,
                available_bytes: avail,
                deduped_from: dup,
            }
        })
        .collect();
    out.sort_by(|a, b| a.mount_point.cmp(&b.mount_point));
    out
}

// ---------- Состояние между вызовами ----------

pub struct MetricsState {
    previous_cpu: Mutex<Option<(CpuTimes, Vec<CpuTimes>)>>,
    previous_net: Mutex<Option<(u64, u64, Instant)>>,
}

impl Default for MetricsState {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsState {
    pub fn new() -> Self {
        Self {
            previous_cpu: Mutex::new(None),
            previous_net: Mutex::new(None),
        }
    }
}

fn delta_percent(prev: CpuTimes, cur: CpuTimes) -> Option<f32> {
    let idle = cur.idle_all.checked_sub(prev.idle_all)?;
    let total = cur.total.checked_sub(prev.total)?;
    if total == 0 {
        return None;
    }
    let busy = total.saturating_sub(idle);
    Some(((busy as f64 / total as f64) * 100.0) as f32)
}

// ---------- statvfs / uname ----------

/// statvfs(2) через libc: единственное место в crate с `unsafe` — обёртка над
/// системным вызовом, структуры инициализируются занулением и проверяем код возврата.
#[cfg(unix)]
fn statvfs_bytes(mount_point: &str) -> Option<(u64, u64, u64)> {
    use std::os::unix::ffi::OsStrExt;
    let c_path = std::ffi::CString::new(mount_point.as_bytes()).ok()?;
    // SAFETY: st полностью перезаписывается statvfs(2) при успехе; при ошибке (rc != 0)
    // мы не читаем поля. Указатель c_path валиден до конца вызова.
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statvfs(c_path.as_ptr(), &mut st) };
    if rc != 0 {
        return None;
    }
    let block = if st.f_frsize != 0 { st.f_frsize as u64 } else { st.f_bsize as u64 };
    let total = (st.f_blocks as u64).saturating_mul(block);
    let free = (st.f_bfree as u64).saturating_mul(block);
    let avail = (st.f_bavail as u64).saturating_mul(block);
    let used = total.saturating_sub(free);
    Some((total, used, avail))
}

#[cfg(not(unix))]
fn statvfs_bytes(_mount_point: &str) -> Option<(u64, u64, u64)> {
    None
}

/// Версия ядра из procfs — без uname(2) и unsafe.
fn kernel_release() -> Option<String> {
    std::fs::read_to_string("/proc/sys/kernel/osrelease").ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Имя узла из procfs — без uname(2) и unsafe.
fn nodename() -> Option<String> {
    std::fs::read_to_string("/proc/sys/kernel/hostname").ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn read_hwmon_temperature() -> Option<f32> {
    let hwmon = Path::new("/sys/class/hwmon");
    let entries = std::fs::read_dir(hwmon).ok()?;
    let mut best: Option<(u8, f32)> = None; // (приоритет, °C); меньше — лучше
    for entry in entries.flatten() {
        let dir = entry.path();
        let name = std::fs::read_to_string(dir.join("name")).unwrap_or_default().trim().to_lowercase();
        let mut cpuish = ["cpu", "coretemp", "k10temp", "tctl", "tdie", "cpu_thermal", "zen"]
            .iter()
            .any(|k| name.contains(k));
        for index in 1..=16 {
            let Ok(input) = std::fs::read_to_string(dir.join(format!("temp{index}_input"))) else { continue };
            let Ok(milli) = input.trim().parse::<f64>() else { continue };
            let celsius = (milli / 1000.0) as f32;
            if !(5.0..125.0).contains(&celsius) {
                continue;
            }
            let label = std::fs::read_to_string(dir.join(format!("temp{index}_label"))).unwrap_or_default().trim().to_lowercase();
            if ["cpu", "core", "package", "ccd", "tdie", "tctl"].iter().any(|k| label.contains(k)) {
                cpuish = true;
            }
            let prio = if cpuish { 0 } else { 10 };
            if best.map(|(p, _)| prio < p).unwrap_or(true) {
                best = Some((prio, celsius));
            }
        }
    }
    best.map(|(_, c)| c)
}

// ---------- Сборка снимка ----------

pub fn collect(state: &MetricsState) -> Snapshot {
    let mut partial: Vec<String> = Vec::new();

    // CPU
    let (cpu_total_percent, cpu_per_core) = match std::fs::read_to_string("/proc/stat") {
        Ok(raw) => {
            let (global, cores) = parse_proc_stat(&raw);
            let mut prev = state.previous_cpu.lock().unwrap();
            let (total_pct, core_pcts) = match (global, prev.as_ref()) {
                (Some(cur), Some((prev_global, prev_cores))) => {
                    let total_pct = delta_percent(*prev_global, cur);
                    let core_pcts: Vec<Option<f32>> = cores
                        .iter()
                        .zip(prev_cores.iter())
                        .map(|(c, p)| delta_percent(*p, *c))
                        .collect();
                    (total_pct, core_pcts)
                }
                (Some(cur), None) => (None, vec![None; cores.len()]),
                (None, _) => (None, Vec::new()),
            };
            if let Some(cur) = global {
                *prev = Some((cur, cores));
            }
            (total_pct, core_pcts)
        }
        Err(_) => (None, Vec::new()),
    };
    if cpu_total_percent.is_none() {
        partial.push("CPU (нужен второй замер)".to_string());
    }

    // Память
    let (memory_used, memory_total, swap_used, swap_total) = match std::fs::read_to_string("/proc/meminfo") {
        Ok(raw) => {
            let m = parse_meminfo(&raw);
            let total = m.get("MemTotal").map(|kb| kb * 1024);
            let avail = m.get("MemAvailable").map(|kb| kb * 1024);
            let used = match (total, avail) {
                (Some(t), Some(a)) => Some(t.saturating_sub(a)),
                _ => None,
            };
            let stotal = m.get("SwapTotal").map(|kb| kb * 1024);
            let sfree = m.get("SwapFree").map(|kb| kb * 1024);
            let sused = match (stotal, sfree) {
                (Some(t), Some(f)) => Some(t.saturating_sub(f)),
                _ => None,
            };
            (used, total, sused, stotal)
        }
        Err(_) => (None, None, None, None),
    };
    if memory_total.is_none() {
        partial.push("память".to_string());
    }

    // Диски (statvfs), без двойного счёта
    let disks = match std::fs::read_to_string("/proc/mounts") {
        Ok(raw) => {
            let entries = parse_mounts(&raw);
            let mut sizes: HashMap<String, (u64, u64, u64)> = HashMap::new();
            for e in &entries {
                if let Some(bytes) = statvfs_bytes(&e.mount_point) {
                    sizes.insert(e.mount_point.clone(), bytes);
                }
            }
            dedupe_mounts(entries, &sizes)
        }
        Err(_) => Vec::new(),
    };
    let disk_total: u64 = disks.iter().map(|d| d.total_bytes).sum();
    let disk_used: u64 = disks.iter().map(|d| d.used_bytes).sum();
    let (disk_used_bytes, disk_total_bytes) = if disks.is_empty() { (None, None) } else { (Some(disk_used), Some(disk_total)) };
    if disks.is_empty() {
        partial.push("диски".to_string());
    }

    // Сеть: скорость за интервал между вызовами (кроме loopback)
    let (net_rx, net_tx) = match std::fs::read_to_string("/proc/net/dev") {
        Ok(raw) => {
            let ifaces = parse_net_dev(&raw);
            let rx_sum: u64 = ifaces.iter().map(|(_, rx, _)| *rx).sum();
            let tx_sum: u64 = ifaces.iter().map(|(_, _, tx)| *tx).sum();
            let mut prev = state.previous_net.lock().unwrap();
            let now = Instant::now();
            let rates = match prev.as_ref() {
                Some((prx, ptx, when)) => {
                    let dt = now.duration_since(*when).as_secs_f64();
                    if dt > 0.05 {
                        // счётчики могли обнулиться после перезагрузки — отрицательная дельта трактуем как 0
                        let rxr = (rx_sum as i128 - *prx as i128).max(0) as f64 / dt;
                        let txr = (tx_sum as i128 - *ptx as i128).max(0) as f64 / dt;
                        (Some(rxr), Some(txr))
                    } else {
                        (None, None)
                    }
                }
                None => (None, None),
            };
            *prev = Some((rx_sum, tx_sum, now));
            rates
        }
        Err(_) => (None, None),
    };
    if net_rx.is_none() {
        partial.push("скорость сети (нужен второй замер)".to_string());
    }

    // Прочее
    let uptime_seconds = std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|raw| raw.split_whitespace().next()?.parse::<f64>().ok())
        .map(|v| v as u64);
    let os_name = util::read_os_release().get("PRETTY_NAME").cloned();
    let kernel = kernel_release();
    let hostname = nodename();
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").ok().filter(|s| !s.is_empty()).map(|d| {
        match std::env::var("XDG_SESSION_TYPE").ok().as_deref() {
            Some(t) => format!("{d} ({t})"),
            None => d,
        }
    });
    let temperature = read_hwmon_temperature();
    if temperature.is_none() {
        partial.push("датчик температуры".to_string());
    }

    Snapshot {
        cpu_total_percent,
        cpu_per_core,
        memory_used_bytes: memory_used,
        memory_total_bytes: memory_total,
        swap_used_bytes: swap_used,
        swap_total_bytes: swap_total,
        disk_used_bytes,
        disk_total_bytes,
        disks,
        network_rx_bytes_per_sec: net_rx,
        network_tx_bytes_per_sec: net_tx,
        temperature_celsius: temperature,
        uptime_seconds,
        hostname,
        kernel,
        os_name,
        desktop,
        updated_at_utc: util::now_rfc3339(),
        partial,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_stat_parses_global_and_cores() {
        let raw = "cpu  100 0 50 800 10 0 5 0 0 0\ncpu0 60 0 30 400 5 0 2 0 0 0\ncpu1 40 0 20 400 5 0 3 0 0 0\nintr 123\n";
        let (global, cores) = parse_proc_stat(raw);
        let g = global.expect("global cpu line");
        assert_eq!(g.idle_all, 810);
        assert_eq!(g.total, 965);
        assert_eq!(cores.len(), 2);
        assert_eq!(cores[0].total, 497);
    }

    #[test]
    fn meminfo_parses_kb() {
        let raw = "MemTotal:       32768000 kB\nMemAvailable:   16384000 kB\nSwapTotal:       8388608 kB\nSwapFree:        4194304 kB\n";
        let m = parse_meminfo(raw);
        assert_eq!(m.get("MemTotal"), Some(&32768000));
        assert_eq!(m.get("SwapFree"), Some(&4194304));
    }

    #[test]
    fn net_dev_skips_loopback() {
        let raw = "Inter-|   Receive  |  Transmit\n face |bytes packets errs drop fifo frame compressed multicast|bytes packets errs drop fifo colls carrier compressed\n    lo: 1000 10 0 0 0 0 0 0 2000 20 0 0 0 0 0 0\n  eth0: 5000 50 0 0 0 0 0 0 7000 70 0 0 0 0 0 0\n";
        let v = parse_net_dev(raw);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0], ("eth0".to_string(), 5000, 7000));
    }

    #[test]
    fn mounts_filters_and_dedupes() {
        let raw = "/dev/nvme0n1p2 / btrfs rw,relatime 0 0\n/dev/nvme0n1p2 /home btrfs rw,relatime 0 0\n/dev/nvme0n1p2 /var btrfs rw,relatime 0 0\ntmpfs /run tmpfs rw 0 0\noverlay /var/lib/docker/overlay overlay rw 0 0\n/dev/sdb1 /mnt/usb vfat rw 0 0\n";
        let entries = parse_mounts(raw);
        assert_eq!(entries.len(), 4); // tmpfs и overlay отфильтрованы
        let mut sizes = HashMap::new();
        sizes.insert("/".to_string(), (1000u64, 400u64, 600u64));
        sizes.insert("/home".to_string(), (1000, 400, 600));
        sizes.insert("/var".to_string(), (1000, 400, 600));
        sizes.insert("/mnt/usb".to_string(), (20000, 5000, 15000));
        let disks = dedupe_mounts(entries, &sizes);
        assert_eq!(disks.len(), 2);
        let root = disks.iter().find(|d| d.mount_point == "/").unwrap();
        assert_eq!(root.deduped_from, vec!["/home".to_string(), "/var".to_string()]);
        assert_eq!(root.total_bytes, 1000);
    }

    #[test]
    fn os_release_parse() {
        let mut m = HashMap::new();
        util::parse_os_release("NAME=\"CachyOS\"\nPRETTY_NAME=\"CachyOS Linux\"\nID=cachyos\n", &mut m);
        assert_eq!(m.get("PRETTY_NAME").unwrap(), "CachyOS Linux");
    }
}
