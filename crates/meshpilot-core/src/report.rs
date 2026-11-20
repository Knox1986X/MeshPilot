use serde::Serialize;

pub const SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Serialize)]
pub struct StatusReport {
    pub schema_version: u8,
    pub read_only: bool,
    pub os_id: Option<String>,
    /// None означает отсутствие наблюдения, а не отрицательный результат.
    pub online: Option<bool>,
    pub dns: Option<String>,
    pub updatable_apps: Option<u32>,
    pub security_fixes: Option<u32>,
    pub last_success_utc: Option<String>,
}

impl StatusReport {
    pub fn unmeasured(os_id: Option<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            read_only: true,
            os_id,
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
