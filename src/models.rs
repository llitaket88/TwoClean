// Модели данных для TwoClear

#[derive(Debug, Clone)]
pub struct InstalledVersion {
    pub name: String,
    pub version: String,
    pub version_int: u64,
    pub uuid: String,
    #[allow(dead_code)]
    pub location: String,
    pub install_date: String,
    pub size: u64,
    pub selected: bool,
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub path: String,
    pub uuid: String,
    pub display_name: String,
    pub size: u64,
    pub selected: bool,
}

#[derive(Debug, Clone)]
pub struct InfoBase {
    pub name: String,
    pub uuid: String,
    pub version: Option<String>,
    pub connection: String,
    pub is_file_base: bool,
    pub size: u64,
    pub selected: bool,
}
