//! Reading and writing the kernel's control files under the port's root.

use std::path::{Path, PathBuf};

pub struct Sysfs<'a> {
    root: &'a Path,
}

impl<'a> Sysfs<'a> {
    pub fn new(root: &'a Path) -> Self {
        Sysfs { root }
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel.trim_start_matches('/'))
    }

    /// The trimmed content of a file, or None when the file does not exist (feature not exposed).
    pub fn read(&self, rel: &str) -> Option<String> {
        std::fs::read_to_string(self.path(rel)).ok().map(|s| s.trim().to_string())
    }

    pub fn read_num<T: std::str::FromStr>(&self, rel: &str) -> Option<T> {
        self.read(rel)?.parse().ok()
    }

    pub fn exists(&self, rel: &str) -> bool {
        self.path(rel).exists()
    }

    pub fn write(&self, rel: &str, value: &str) -> anyhow::Result<()> {
        let p = self.path(rel);
        std::fs::write(&p, value).map_err(|e| anyhow::anyhow!("writing {}: {e}", p.display()))
    }

    /// The directory (relative to the root) of the hwmon device with this name.
    pub fn hwmon(&self, name: &str) -> Option<String> {
        let dir = self.path("sys/class/hwmon");
        let mut entries: Vec<_> = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.file_name()).collect();
        entries.sort();
        entries.into_iter().map(|n| format!("sys/class/hwmon/{}", n.to_string_lossy()))
            .find(|rel| self.read(&format!("{rel}/name")).as_deref() == Some(name))
    }
}
