// Модели данных

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub path: String,
    pub uuid: String,
    pub connection: String,
    pub display_name: String,
    pub size: u64,
    pub selected: bool,
}

#[derive(Debug, Clone)]
pub struct InfoBase {
    pub name: String,
    pub uuid: String,
    pub connection: String,
}
