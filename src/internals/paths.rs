use semver::Version;
use std::path::{Path, PathBuf};

pub const PRISMUP_DIR: &str = ".prismup";
pub const PRISMUP_CACHE_DIR: &str = ".cache/prismup";
pub const RELEASES_CACHE_FILENAME: &str = "releases.json";
pub const PART_EXTENSION: &str = "part";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    root: PathBuf,
    bin_dir: PathBuf,
    versions_dir: PathBuf,
    cache_dir: PathBuf,
    releases_cache: PathBuf,
}

impl Layout {
    pub fn from_home(home_dir: &Path) -> Self {
        let root = home_dir.join(PRISMUP_DIR);
        let bin_dir = root.join("bin");
        let versions_dir = root.join("prism");
        let cache_dir = home_dir.join(PRISMUP_CACHE_DIR);
        let releases_cache = cache_dir.join(RELEASES_CACHE_FILENAME);
        Layout {
            root,
            bin_dir,
            versions_dir,
            cache_dir,
            releases_cache,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn bin_dir(&self) -> &Path {
        &self.bin_dir
    }

    pub fn versions_dir(&self) -> &Path {
        &self.versions_dir
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn releases_cache(&self) -> &Path {
        &self.releases_cache
    }

    pub fn version_dir(&self, version: &Version) -> PathBuf {
        self.versions_dir.join(version.to_string())
    }

    pub fn version_prism_binary(&self, version: &Version) -> PathBuf {
        self.version_dir(version).join("prism")
    }

    pub fn version_link(&self, version: &Version) -> PathBuf {
        self.bin_dir.join(format!("prism-{}", version))
    }

    pub fn current_prism_binary(&self) -> PathBuf {
        self.bin_dir.join("prism")
    }

    pub fn archive_path(&self, archive_filename: &str) -> PathBuf {
        self.cache_dir.join(archive_filename)
    }
}

pub fn part_path_of(path: &Path) -> PathBuf {
    let mut part_path = path.as_os_str().to_os_string();
    part_path.push(".");
    part_path.push(PART_EXTENSION);
    PathBuf::from(part_path)
}

#[cfg(test)]
pub fn temp_home(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let home_dir = std::env::temp_dir().join(format!(
        "prismup-test-{}-{}-{}",
        tag,
        std::process::id(),
        unique
    ));
    std::fs::create_dir_all(&home_dir).expect("the temporary home directory should be created");
    home_dir
}

#[cfg(test)]
pub fn temp_layout(tag: &str) -> Layout {
    let home_dir = temp_home(tag);
    let layout = Layout::from_home(&home_dir);
    for directory in [layout.cache_dir(), layout.bin_dir(), layout.versions_dir()] {
        std::fs::create_dir_all(directory).expect("the PrismUp directories should be created");
    }
    layout
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_home_builds_the_expected_directories() {
        let layout = Layout::from_home(Path::new("/home/account"));
        assert_eq!(layout.root(), Path::new("/home/account/.prismup"));
        assert_eq!(layout.bin_dir(), Path::new("/home/account/.prismup/bin"));
        assert_eq!(
            layout.versions_dir(),
            Path::new("/home/account/.prismup/prism")
        );
        assert_eq!(
            layout.cache_dir(),
            Path::new("/home/account/.cache/prismup")
        );
        assert_eq!(
            layout.releases_cache(),
            Path::new("/home/account/.cache/prismup/releases.json")
        );
    }

    #[test]
    fn version_paths_depend_on_the_given_version() {
        let layout = Layout::from_home(Path::new("/home/account"));
        let version = Version::new(0, 22, 0);
        assert_eq!(
            layout.version_dir(&version),
            Path::new("/home/account/.prismup/prism/0.22.0")
        );
        assert_eq!(
            layout.version_prism_binary(&version),
            Path::new("/home/account/.prismup/prism/0.22.0/prism")
        );
        assert_eq!(
            layout.version_link(&version),
            Path::new("/home/account/.prismup/bin/prism-0.22.0")
        );
        assert_eq!(
            layout.current_prism_binary(),
            Path::new("/home/account/.prismup/bin/prism")
        );
    }

    #[test]
    fn part_path_is_the_path_with_a_part_suffix() {
        assert_eq!(
            part_path_of(Path::new("/tmp/prismup/prism-0.22.0.tar.gz")),
            Path::new("/tmp/prismup/prism-0.22.0.tar.gz.part")
        );
        assert_eq!(
            part_path_of(Path::new("/tmp/prismup/releases.json")),
            Path::new("/tmp/prismup/releases.json.part")
        );
    }
}
