//! What outlives a run: the theme, the explorer's width and whether it is open, the growth
//! budget and the vaults you have opened — every setting the dialog has.
//!
//! One file of `key=value` lines under the desktop's config directory. A key may appear
//! more than once, which is the whole of the recent list — no format, no parser, and
//! nothing to go wrong that deleting the file does not already fix.
//!
//! The file is read once, into this, and written back whenever a setting is changed.
//! Nothing else reads it, so what is on disk and what is in memory cannot disagree.

use std::path::PathBuf;

const THEME: &str = "theme";
const GROWTH: &str = "growth";
const RECENT: &str = "recent";
const PANEL: &str = "panel";
const EXPLORER: &str = "explorer";
/// How many vaults the picker remembers.
const REMEMBERED: usize = 6;

pub struct Settings {
    rows: Vec<(String, String)>,
}

impl Settings {
    pub fn load() -> Settings {
        let text = Settings::file()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .unwrap_or_default();
        let rows = text
            .lines()
            .filter_map(|line| line.split_once('='))
            .map(|(key, value)| (key.trim().to_string(), value.to_string()))
            .collect();
        Settings { rows }
    }

    pub fn theme(&self) -> Option<usize> {
        crate::theme::by_key(&self.one(THEME)?)
    }

    pub fn budget(&self, fallback: u32) -> u32 {
        self.number(GROWTH, fallback)
    }

    pub fn panel(&self, fallback: f64) -> f64 {
        self.number(PANEL, fallback)
    }

    /// Whether the explorer is shown. Anything but `off` is on, since a typo should leave
    /// the panel where it can be seen.
    pub fn explorer(&self) -> bool {
        self.one(EXPLORER).is_none_or(|v| v.trim() != "off")
    }

    pub fn recent(&self) -> Vec<String> {
        self.rows
            .iter()
            .filter(|(k, _)| k == RECENT)
            .map(|(_, v)| v.clone())
            .collect()
    }

    pub fn set_theme(&mut self, key: &str) {
        self.put(THEME, &[key.to_string()]);
    }

    pub fn set_budget(&mut self, ms: u32) {
        self.put(GROWTH, &[ms.to_string()]);
    }

    /// The explorer as it is left: its width, and whether it is open at all.
    pub fn set_panel(&mut self, px: f64, hidden: bool) {
        self.put(PANEL, &[px.round().to_string()]);
        let shown = match hidden {
            true => "off",
            false => "on",
        };
        self.put(EXPLORER, &[shown.to_string()]);
    }

    /// A vault opened moves to the front of the list, and the list stays short.
    pub fn remember(&mut self, vault: &str) -> Vec<String> {
        let mut rows = vec![vault.to_string()];
        rows.extend(self.recent().into_iter().filter(|p| p != vault));
        rows.truncate(REMEMBERED);
        self.put(RECENT, &rows);
        rows
    }

    fn one(&self, key: &str) -> Option<String> {
        self.rows
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    }

    /// A number that was stored, or the fallback when the file is new or the line is junk.
    fn number<T: std::str::FromStr>(&self, key: &str, fallback: T) -> T {
        self.one(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(fallback)
    }

    /// Every line for `key` is replaced by these, in order, and the file is rewritten.
    fn put(&mut self, key: &str, values: &[String]) {
        self.rows.retain(|(k, _)| k != key);
        self.rows
            .extend(values.iter().map(|v| (key.to_string(), v.clone())));
        self.save();
    }

    fn save(&self) {
        let Some(path) = Settings::file() else { return };
        let text: String = self
            .rows
            .iter()
            .map(|(k, v)| format!("{k}={v}\n"))
            .collect();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, text);
    }

    fn file() -> Option<PathBuf> {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(config.join("brain-map").join("settings"))
    }
}
