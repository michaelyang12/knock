//! Cache and history as plain JSON files under `~/.knock/`. Every operation is best-effort:
//! a broken or locked file never stops knock from answering.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CACHE_TTL_SECS: u64 = 30 * 24 * 60 * 60;
const CACHE_MAX_ENTRIES: usize = 500;
const HISTORY_MAX_ENTRIES: usize = 200;

pub fn data_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".knock")
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn read_json<T: DeserializeOwned + Default>(path: &Path) -> T {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Writes through a temp file and rename so concurrent `kn` runs never see a half-written file.
fn write_json<T: Serialize>(path: &Path, value: &T) {
    let Some(dir) = path.parent() else { return };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    let Ok(text) = serde_json::to_string(value) else {
        return;
    };
    if std::fs::write(&tmp, text).is_ok() && std::fs::rename(&tmp, path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    value: String,
    created: u64,
}

pub struct Cache {
    path: PathBuf,
    entries: HashMap<String, CacheEntry>,
}

impl Cache {
    pub fn open(dir: &Path) -> Self {
        let path = dir.join("cache.json");
        let entries = read_json(&path);
        Self { path, entries }
    }

    /// Everything that changes the answer belongs in the key.
    pub fn key(parts: &[&str]) -> String {
        parts.join("\u{1f}")
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .get(key)
            .filter(|e| now().saturating_sub(e.created) < CACHE_TTL_SECS)
            .map(|e| e.value.as_str())
    }

    pub fn put(&mut self, key: String, value: String) {
        let created = now();
        self.entries
            .retain(|_, e| created.saturating_sub(e.created) < CACHE_TTL_SECS);
        if self.entries.len() >= CACHE_MAX_ENTRIES {
            let mut ages: Vec<(String, u64)> = self
                .entries
                .iter()
                .map(|(k, e)| (k.clone(), e.created))
                .collect();
            ages.sort_by_key(|(_, created)| *created);
            let excess = self.entries.len() + 1 - CACHE_MAX_ENTRIES;
            for (k, _) in ages.into_iter().take(excess) {
                self.entries.remove(&k);
            }
        }
        self.entries.insert(key, CacheEntry { value, created });
        write_json(&self.path, &self.entries);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HistoryEntry {
    pub query: String,
    pub command: String,
    pub timestamp: u64,
}

pub struct History {
    path: PathBuf,
}

impl History {
    pub fn open(dir: &Path) -> Self {
        Self {
            path: dir.join("history.json"),
        }
    }

    fn entries(&self) -> Vec<HistoryEntry> {
        read_json(&self.path)
    }

    pub fn add(&self, query: &str, command: &str) {
        let mut entries = self.entries();
        entries.retain(|e| !(e.query == query && e.command == command));
        entries.push(HistoryEntry {
            query: query.to_string(),
            command: command.to_string(),
            timestamp: now(),
        });
        let excess = entries.len().saturating_sub(HISTORY_MAX_ENTRIES);
        entries.drain(..excess);
        write_json(&self.path, &entries);
    }

    /// Newest first.
    pub fn recent(&self, count: usize) -> Vec<HistoryEntry> {
        self.entries().into_iter().rev().take(count).collect()
    }

    /// Newest first.
    pub fn search(&self, term: &str) -> Vec<HistoryEntry> {
        let term = term.to_lowercase();
        self.entries()
            .into_iter()
            .rev()
            .filter(|e| {
                e.query.to_lowercase().contains(&term) || e.command.to_lowercase().contains(&term)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_round_trips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let key = Cache::key(&["standard", "ollama", "llama3.2", "list files"]);
        Cache::open(dir.path()).put(key.clone(), "ls".into());
        assert_eq!(Cache::open(dir.path()).get(&key), Some("ls"));
        assert_eq!(Cache::open(dir.path()).get("other"), None);
    }

    #[test]
    fn expired_cache_entries_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = Cache::open(dir.path());
        cache.entries.insert(
            "old".into(),
            CacheEntry {
                value: "x".into(),
                created: now() - CACHE_TTL_SECS - 1,
            },
        );
        assert_eq!(cache.get("old"), None);
    }

    #[test]
    fn cache_evicts_oldest_when_full() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = Cache::open(dir.path());
        for i in 0..CACHE_MAX_ENTRIES {
            cache.entries.insert(
                format!("k{i}"),
                CacheEntry {
                    value: String::new(),
                    created: now() - 1000 + i as u64,
                },
            );
        }
        cache.put("new".into(), "v".into());
        assert_eq!(cache.entries.len(), CACHE_MAX_ENTRIES);
        assert!(!cache.entries.contains_key("k0"));
        assert!(cache.entries.contains_key("new"));
    }

    #[test]
    fn corrupt_files_read_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("cache.json"), "{not json").unwrap();
        std::fs::write(dir.path().join("history.json"), "[1,2").unwrap();
        assert_eq!(Cache::open(dir.path()).get("k"), None);
        assert!(History::open(dir.path()).recent(5).is_empty());
    }

    #[test]
    fn history_dedupes_caps_and_lists_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let history = History::open(dir.path());
        for i in 0..HISTORY_MAX_ENTRIES + 5 {
            history.add(&format!("q{i}"), &format!("cmd{i}"));
        }
        history.add("q3x", "git status");
        history.add("q3x", "git status");
        let recent = history.recent(3);
        assert_eq!(recent[0].command, "git status");
        assert_eq!(recent[1].command, format!("cmd{}", HISTORY_MAX_ENTRIES + 4));
        assert_eq!(history.entries().len(), HISTORY_MAX_ENTRIES);
        assert_eq!(history.search("GIT STATUS").len(), 1);
    }
}
