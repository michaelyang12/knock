use std::path::PathBuf;

pub struct Cache {
    db: sled::Db,
}

impl Cache {
    fn cache_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".knock").join("cache")
    }

    pub fn load() -> Self {
        let db = sled::open(Self::cache_path()).expect("Failed to open cache database");
        Self { db }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.db
            .get(key)
            .ok()
            .flatten()
            .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
    }

    pub fn insert(&self, key: String, response: String) {
        let _ = self.db.insert(key, response.as_bytes());
    }

    pub fn generate_key(query: &str, os: &str, shell: &str, mode: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        os.hash(&mut hasher);
        shell.hash(&mut hasher);
        mode.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}
