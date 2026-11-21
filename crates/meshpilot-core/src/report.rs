use serde::Serialize;

pub const SCHEMA_VERSION: u8 = 1;

/// Полный результат одного чтения. В отчёте None вместо частичного или вымышленного замера.
#[derive(Debug, Serialize)]
pub struct RamObservation {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub source: &'static str,
    /// RFC 3339, UTC, время измерения, не время отрисовки.
    pub observed_at_utc: String,
}

/// Загрузка по двум чтениям; интервал монотонный, UTC относится к концу замера.
#[derive(Debug, Serialize)]
pub struct CpuObservation {
    pub usage_percent: f64,
    pub interval_ms: u64,
    pub busy_ticks: u64,
    pub total_ticks: u64,
    pub source: &'static str,
    pub observed_at_utc: String,
}

#[derive(Debug, Serialize)]
pub struct StatusReport {
    pub schema_version: u8,
    pub read_only: bool,
    /// Буквальное ID из os-release, а не имя обнаруженного дистрибутива.
    pub os_id: Option<String>,
    pub cachyos: Option<bool>,
    /// None означает отсутствие наблюдения, а не отрицательный результат.
    pub cpu: Option<CpuObservation>,
    pub ram: Option<RamObservation>,
    pub online: Option<bool>,
    pub dns: Option<String>,
    pub updatable_apps: Option<u32>,
    pub security_fixes: Option<u32>,
    pub last_success_utc: Option<String>,
}

impl StatusReport {
    pub fn unmeasured(os_id: Option<String>, cachyos: Option<bool>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            read_only: true,
            os_id,
            cachyos,
            cpu: None,
            ram: None,
            online: None,
            dns: None,
            updatable_apps: None,
            security_fixes: None,
            last_success_utc: None,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Finding {
    Detected,
    Missing,
    Unsupported,
    Unknown,
}

#[derive(Debug, Serialize)]
pub struct Check {
    pub id: &'static str,
    pub required: bool,
    pub finding: Finding,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub schema_version: u8,
    pub read_only: bool,
    pub checks: Vec<Check>,
}

impl DoctorReport {
    pub fn ready(&self) -> bool {
        self.checks
            .iter()
            .all(|c| !c.required || c.finding == Finding::Detected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_measurements_are_explicitly_null_in_v1() {
        let json = serde_json::to_value(StatusReport::unmeasured(None, None)).unwrap();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json.get("cpu"), Some(&serde_json::Value::Null));
        assert_eq!(json.get("ram"), Some(&serde_json::Value::Null));
    }
}
