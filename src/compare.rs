// stub - will be implemented next
use crate::parser::EnvFile;

pub struct DriftReport {
    pub files: Vec<EnvFile>,
    pub all_keys: Vec<String>,
}

impl DriftReport {
    pub fn has_drift(&self) -> bool {
        false
    }
}

pub fn compare_files(_files: &[EnvFile]) -> DriftReport {
    DriftReport {
        files: vec![],
        all_keys: vec![],
    }
}
