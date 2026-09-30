use crate::internals::network::{SHA256_PREFIX, download_backoff, get_sha256};
use crate::internals::paths::Layout;
use crate::internals::platform::Platform;
use crate::internals::versions::{
    expected_sha256_for, get_available_prism_versions, get_prism_installed_versions,
    get_released_prism_versions, get_versions_list, to_semver,
};
use crate::types::{NetworkOptions, Release};
use colored::Colorize;
use flate2::read::GzDecoder;
use semver::Version;
use sha256::try_digest;
use std::env;
use std::fs::{self, File};
use std::io::ErrorKind;
use std::os::unix::fs::symlink;
use std::path::Path;
use tar::Archive;
use tokio::fs::create_dir_all;

pub mod args;
pub mod network;
pub mod paths;
pub mod platform;
pub mod versions;

pub struct PrismUp<'a> {
    client: &'a reqwest::Client,
    layout: &'a Layout,
    platform: &'a Platform,
    releases: &'a [Release],
    options: NetworkOptions,
}

impl<'a> PrismUp<'a> {
    pub fn new(
        client: &'a reqwest::Client,
        layout: &'a Layout,
        platform: &'a Platform,
        releases: &'a [Release],
        options: NetworkOptions,
    ) -> Self {
        PrismUp {
            client,
            layout,
            platform,
            releases,
            options,
        }
    }

    pub fn current_version(&self) -> Option<Version> {
        get_current_prism_version(self.layout)
    }

    pub fn installed_versions(&self) -> Option<Vec<Version>> {
        get_prism_installed_versions(self.layout.versions_dir())
    }

    pub fn released_versions(&self) -> Vec<Version> {
        get_released_prism_versions(self.releases)
    }

    pub fn available_versions(&self) -> Vec<Version> {
        get_available_prism_versions(self.releases, self.platform)
    }

    pub fn versions_list(&self) {
        get_versions_list(
            &self.released_versions(),
            &self.available_versions(),
            &self.installed_versions().unwrap_or_default(),
            self.current_version().as_ref(),
            self.platform,
        );
    }

    pub async fn upgrade(&self) -> Result<(), String> {
        let available_prism_versions = self.available_versions();
        let latest_prism_version = available_prism_versions
            .last()
            .ok_or("No Prism version released yet.".to_string())?;
        if let Some(newest_version) = self
            .released_versions()
            .last()
            .filter(|newest_version| *newest_version != latest_prism_version)
        {
            println!(
                "Prism version {} has no binary for {}, so Prism version {} is the latest installable one.",
                newest_version,
                self.platform.target(),
                latest_prism_version.to_string().bold()
            );
        }
        if self
            .installed_versions()
            .is_some_and(|installed_versions| installed_versions.contains(latest_prism_version))
        {
            println!(
                "The latest Prism version {} is already installed.",
                latest_prism_version.to_string().bold()
            );
        } else {
            println!(
                "Installation of the latest Prism compiler ({}).",
                latest_prism_version.to_string().bold()
            );
            install_prism_version(
                self.client,
                self.layout,
                self.platform,
                self.releases,
                latest_prism_version,
                self.options,
            )
            .await?;
        }
        set_current_prism_version(self.layout, latest_prism_version)
    }

    pub async fn install(&self, version: &Version) -> Result<(), String> {
        if self
            .installed_versions()
            .is_some_and(|installed_versions| installed_versions.contains(version))
        {
            println!("Prism {} already installed.", version.to_string().bold());
            return Ok(());
        }
        if !self.available_versions().contains(version) {
            return Err(self.no_prism_binary_error(version));
        }
        println!(
            "Installation of Prism compiler version {}...",
            version.to_string().bold()
        );
        install_prism_version(
            self.client,
            self.layout,
            self.platform,
            self.releases,
            version,
            self.options,
        )
        .await
    }

    pub async fn set(&self, version: &Version) -> Result<(), String> {
        if !self
            .installed_versions()
            .unwrap_or_default()
            .contains(version)
        {
            println!(
                "Prism version {} needs to be installed before being set.",
                version.to_string().bold()
            );
            self.install(version).await?;
        }
        set_current_prism_version(self.layout, version)
    }

    pub fn remove(&self, version: &Version) -> Result<(), String> {
        prism_version_remove(
            self.layout,
            &self.released_versions(),
            &self.installed_versions().unwrap_or_default(),
            version,
        )
    }

    fn no_prism_binary_error(&self, version: &Version) -> String {
        if self.released_versions().contains(version) {
            format!(
                "Prism version {} is released but has no binary for {}.",
                version,
                self.platform.target()
            )
        } else {
            format!("Sorry but {} is not a Prism version released.", version)
        }
    }
}

pub fn prism_version_remove(
    layout: &Layout,
    released_versions: &[Version],
    installed_versions: &[Version],
    version: &Version,
) -> Result<(), String> {
    if !released_versions.contains(version) {
        return Err(format!(
            "{} is not even an available version of Prism.",
            version
        ));
    }
    if !installed_versions.contains(version) {
        return Err(format!("{} is not an installed version of Prism.", version));
    }
    if get_current_prism_version(layout).as_ref() == Some(version) {
        return Err(format!(
            "Prism version {} is your current Prism compiler, please set another Prism version before removing this one.",
            version
        ));
    }
    remove_file_if_existing(&layout.version_link(version))?;
    remove_dir_all_if_existing(&layout.version_dir(version))?;
    println!("Prism version {} removed.", version.to_string().bold());
    Ok(())
}

pub fn get_current_prism_version(layout: &Layout) -> Option<Version> {
    let current_prism_binary = layout.current_prism_binary();
    let target = fs::read_link(&current_prism_binary).ok()?;
    let version_directory = target.parent()?.file_name()?.to_string_lossy().into_owned();
    to_semver(&version_directory).ok()
}

pub fn set_current_prism_version(layout: &Layout, version: &Version) -> Result<(), String> {
    if get_current_prism_version(layout).as_ref() == Some(version) {
        println!(
            "Prism version {} is already set as your current Prism compiler.",
            version.to_string().bold()
        );
        return Ok(());
    }
    let prism_binary = layout.version_prism_binary(version);
    if !prism_binary.exists() {
        return Err(format!(
            "Prism version {} is not installed, there is nothing to set.",
            version
        ));
    }
    let current_prism_binary = layout.current_prism_binary();
    remove_file_if_existing(&current_prism_binary)?;
    symlink(&prism_binary, &current_prism_binary).map_err(|e| {
        format!(
            "Failed to create the symlink '{}': {}",
            current_prism_binary.display(),
            e
        )
    })?;
    println!(
        "Set Prism version {} as your current Prism compiler.",
        version.to_string().bold()
    );
    Ok(())
}

pub fn clear_cache(layout: &Layout) -> Result<(), String> {
    let entries = match fs::read_dir(layout.cache_dir()) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            println!("{}", "PrismUp cache is already empty.".green());
            return Ok(());
        }
        Err(e) => {
            return Err(format!(
                "Failed to read the cache directory '{}': {}",
                layout.cache_dir().display(),
                e
            ));
        }
    };
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read the cache directory: {}", e))?;
        let path = entry.path();
        let remove_result = match entry
            .file_type()
            .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?
        {
            file_type if file_type.is_dir() => fs::remove_dir_all(&path),
            _ => fs::remove_file(&path),
        };
        remove_result.map_err(|e| format!("Failed to remove '{}': {}", path.display(), e))?;
    }
    println!("{}", "PrismUp cache cleared.".green());
    Ok(())
}

pub fn uninstall(layout: &Layout) -> Result<(), String> {
    println!("Removing '{}'...", layout.root().display());
    remove_dir_all_if_existing(layout.root())?;
    println!("Removing '{}'...", layout.cache_dir().display());
    remove_dir_all_if_existing(layout.cache_dir())?;
    if let Ok(exe_path) = env::current_exe() {
        println!("Removing the prismup binary at '{}'...", exe_path.display());
        remove_file_if_existing(&exe_path)?;
    }
    println!("You can remove '$HOME/.prismup/bin/' from your PATH.");
    Ok(())
}

pub async fn make_prismup_directories(layout: &Layout) -> Result<(), String> {
    for directory in [layout.cache_dir(), layout.bin_dir(), layout.versions_dir()] {
        create_dir_all(directory)
            .await
            .map_err(|e| format!("Failed to create '{}': {}", directory.display(), e))?;
    }
    Ok(())
}

pub async fn install_prism_version(
    client: &reqwest::Client,
    layout: &Layout,
    platform: &Platform,
    releases: &[Release],
    version: &Version,
    options: NetworkOptions,
) -> Result<(), String> {
    let archive_filename = platform.archive_filename(version);
    let archive_url = platform.archive_url(version);
    let archive_path = layout.archive_path(&archive_filename);
    let expected_sha256 = expected_sha256(client, platform, releases, version, options).await?;
    if archive_path.exists() && is_file_integrity_ok(&expected_sha256, &archive_path)? {
        println!(
            "Using the already downloaded archive of Prism {}.",
            version.to_string().bold()
        );
    } else if options.offline {
        return Err(format!(
            "The archive of Prism {} is not in the PrismUp cache, it cannot be installed while offline.",
            version
        ));
    } else {
        download_backoff(client, &archive_url, &archive_path).await?;
        if !is_file_integrity_ok(&expected_sha256, &archive_path)? {
            let _ = fs::remove_file(&archive_path);
            return Err(format!(
                "SHA256 integrity check failed for '{}'.",
                archive_filename
            ));
        }
    }
    unpack_prism_archive(&archive_path, layout, platform, version)
}

async fn expected_sha256(
    client: &reqwest::Client,
    platform: &Platform,
    releases: &[Release],
    version: &Version,
    options: NetworkOptions,
) -> Result<String, String> {
    match expected_sha256_for(releases, platform, version) {
        Some(digest) => Ok(digest),
        None if options.offline => Err(format!(
            "The SHA256 of the Prism {} archive is unknown while offline.",
            version
        )),
        None => {
            let sha256_url = format!("{}.sha256", platform.archive_url(version));
            println!(
                "Getting the SHA256 of the Prism {} archive from '{}'.",
                version, sha256_url
            );
            get_sha256(client, &sha256_url).await
        }
    }
}

fn unpack_prism_archive(
    archive_path: &Path,
    layout: &Layout,
    platform: &Platform,
    version: &Version,
) -> Result<(), String> {
    let archive_file = File::open(archive_path)
        .map_err(|e| format!("Failed to open '{}': {}", archive_path.display(), e))?;
    let mut archive = Archive::new(GzDecoder::new(archive_file));
    archive.unpack(layout.versions_dir()).map_err(|e| {
        format!(
            "Failed to unpack '{}' into '{}': {}",
            archive_path.display(),
            layout.versions_dir().display(),
            e
        )
    })?;
    let extracted_dir = layout.versions_dir().join(platform.asset_name(version));
    let version_dir = layout.version_dir(version);
    remove_dir_all_if_existing(&version_dir)?;
    fs::rename(&extracted_dir, &version_dir).map_err(|e| {
        format!(
            "Failed to move '{}' to '{}': {}",
            extracted_dir.display(),
            version_dir.display(),
            e
        )
    })?;
    let version_link = layout.version_link(version);
    remove_file_if_existing(&version_link)?;
    symlink(layout.version_prism_binary(version), &version_link).map_err(|e| {
        format!(
            "Failed to create the symlink '{}': {}",
            version_link.display(),
            e
        )
    })
}

pub fn is_file_integrity_ok(right_sha256_hash: &str, filepath: &Path) -> Result<bool, String> {
    let file_sha256_hash = try_digest(filepath).map_err(|e| {
        format!(
            "Failed to compute the SHA256 of '{}': {}",
            filepath.display(),
            e
        )
    })?;
    Ok((SHA256_PREFIX.to_owned() + &file_sha256_hash) == right_sha256_hash)
}

fn remove_file_if_existing(path: &Path) -> Result<(), String> {
    if fs::symlink_metadata(path).is_ok() {
        fs::remove_file(path)
            .map_err(|e| format!("Failed to remove '{}': {}", path.display(), e))?;
    }
    Ok(())
}

fn remove_dir_all_if_existing(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|e| format!("Failed to remove '{}': {}", path.display(), e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::internals::network::web_client;
    use crate::internals::paths::{temp_home, temp_layout};
    use crate::internals::versions::fixture_releases;

    const EMPTY_SHA256: &str =
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    fn install_stub(layout: &Layout, version: &Version) {
        fs::create_dir_all(layout.bin_dir()).expect("the bin directory should exist");
        fs::create_dir_all(layout.version_dir(version)).expect("a version directory should exist");
        fs::write(layout.version_prism_binary(version), "")
            .expect("the prism binary should be written");
        symlink(
            layout.version_prism_binary(version),
            layout.version_link(version),
        )
        .expect("the version link should be created");
    }

    #[tokio::test]
    async fn prismup_knows_the_versions_available_for_the_platform() {
        let releases = fixture_releases();
        let client = web_client().await.expect("a web client should be built");
        let layout = temp_layout("platform-aware");
        let linux = Platform::from_uname("Linux", "x86_64").expect("Linux on x86_64 is supported");
        let mac = Platform::from_uname("Darwin", "arm64").expect("macOS on arm64 is supported");
        let online = NetworkOptions {
            force_refresh: false,
            offline: true,
        };
        let linux_prismup = PrismUp::new(&client, &layout, &linux, &releases, online);
        let mac_prismup = PrismUp::new(&client, &layout, &mac, &releases, online);
        assert_eq!(
            linux_prismup.available_versions(),
            vec![Version::new(0, 22, 0)]
        );
        assert_eq!(
            mac_prismup.available_versions(),
            vec![Version::new(0, 3, 0)]
        );
        assert_eq!(
            linux_prismup.released_versions().last(),
            linux_prismup.available_versions().last()
        );
        assert_ne!(
            mac_prismup.released_versions().last(),
            mac_prismup.available_versions().last()
        );
        assert_eq!(
            linux_prismup
                .no_prism_binary_error(&Version::new(0, 3, 0))
                .to_string(),
            "Prism version 0.3.0 is released but has no binary for x86_64-unknown-linux-gnu."
        );
        assert_eq!(
            mac_prismup
                .no_prism_binary_error(&Version::new(0, 22, 0))
                .to_string(),
            "Prism version 0.22.0 is released but has no binary for aarch64-apple-darwin."
        );
        assert_eq!(
            linux_prismup
                .no_prism_binary_error(&Version::new(0, 99, 0))
                .to_string(),
            "Sorry but 0.99.0 is not a Prism version released."
        );
        let _ = fs::remove_dir_all(layout.root().parent().expect("a home directory"));
    }

    #[test]
    fn is_file_integrity_ok_matches_the_expected_digest() {
        let home_dir = temp_home("integrity");
        let file_path = home_dir.join("archive.tar.gz");
        fs::write(&file_path, "").expect("the file should be written");
        assert!(
            is_file_integrity_ok(EMPTY_SHA256, &file_path).expect("the digest should be computed")
        );
        assert!(
            !is_file_integrity_ok("sha256:0000", &file_path)
                .expect("the digest should be computed")
        );
        let _ = fs::remove_dir_all(&home_dir);
    }

    #[test]
    fn is_file_integrity_ok_fails_on_a_missing_file() {
        let layout = temp_layout("integrity-missing");
        let missing = layout.cache_dir().join("missing.tar.gz");
        assert!(is_file_integrity_ok(EMPTY_SHA256, &missing).is_err());
        let _ = fs::remove_dir_all(layout.root().parent().expect("a home directory"));
    }

    #[test]
    fn set_and_get_current_prism_version() {
        let layout = temp_layout("set-current");
        let version = Version::new(0, 22, 0);
        assert_eq!(get_current_prism_version(&layout), None);
        set_current_prism_version(&layout, &version).expect_err("Prism 0.22.0 is not installed");
        install_stub(&layout, &version);
        set_current_prism_version(&layout, &version).expect("Prism 0.22.0 should be set");
        set_current_prism_version(&layout, &version).expect("setting it twice is not an error");
        assert_eq!(get_current_prism_version(&layout), Some(version));
        let _ = fs::remove_dir_all(layout.root().parent().expect("a home directory"));
    }

    #[test]
    fn get_current_prism_version_reads_the_version_of_the_symlink_target() {
        let home_dir = temp_home("current-from-link");
        let layout = Layout::from_home(&home_dir);
        let version = Version::new(0, 16, 0);
        install_stub(&layout, &version);
        symlink(
            layout.version_prism_binary(&version),
            layout.current_prism_binary(),
        )
        .expect("the current link should be created");
        assert_eq!(get_current_prism_version(&layout), Some(version));
        let _ = fs::remove_dir_all(&home_dir);
    }

    #[test]
    fn prism_version_remove_removes_an_installed_version() {
        let layout = temp_layout("remove-version");
        let version = Version::new(0, 16, 0);
        let other_version = Version::new(0, 22, 0);
        install_stub(&layout, &version);
        install_stub(&layout, &other_version);
        let versions = [version.clone(), other_version.clone()];
        prism_version_remove(&layout, &versions, &versions, &version)
            .expect("Prism 0.16.0 should be removed");
        assert!(!layout.version_dir(&version).exists());
        assert!(!layout.version_link(&version).exists());
        assert!(layout.version_dir(&other_version).exists());
        let _ = fs::remove_dir_all(layout.root().parent().expect("a home directory"));
    }

    #[test]
    fn prism_version_remove_rejects_unknown_and_current_versions() {
        let layout = temp_layout("remove-rejections");
        let current = Version::new(0, 22, 0);
        let unknown = Version::new(0, 3, 0);
        install_stub(&layout, &current);
        symlink(
            layout.version_prism_binary(&current),
            layout.current_prism_binary(),
        )
        .expect("the current link should be created");
        prism_version_remove(
            &layout,
            std::slice::from_ref(&current),
            std::slice::from_ref(&current),
            &unknown,
        )
        .expect_err("Prism 0.3.0 is not released");
        prism_version_remove(&layout, std::slice::from_ref(&unknown), &[], &unknown)
            .expect_err("Prism 0.3.0 is not an installed version");
        prism_version_remove(
            &layout,
            std::slice::from_ref(&current),
            std::slice::from_ref(&current),
            &current,
        )
        .expect_err("Prism 0.22.0 is the current Prism compiler");
        assert!(layout.version_dir(&current).exists());
        let _ = fs::remove_dir_all(layout.root().parent().expect("a home directory"));
    }

    #[test]
    fn clear_cache_removes_files_and_directories() {
        let layout = temp_layout("clear-cache");
        let nested_dir = layout.cache_dir().join("nested");
        fs::create_dir_all(&nested_dir).expect("a nested directory should be created");
        fs::write(nested_dir.join("archive.tar.gz"), "").expect("a file should be written");
        fs::write(layout.cache_dir().join("releases.json"), "[]")
            .expect("a file should be written");
        clear_cache(&layout).expect("the cache should be cleared");
        assert!(layout.cache_dir().is_dir());
        assert_eq!(
            fs::read_dir(layout.cache_dir())
                .expect("an existing cache")
                .count(),
            0
        );
        clear_cache(&layout).expect("clearing an empty cache is not an error");
        let _ = fs::remove_dir_all(layout.root().parent().expect("a home directory"));
    }

    #[test]
    fn clear_cache_accepts_a_missing_cache_directory() {
        let home_dir = temp_home("clear-cache-missing");
        let layout = Layout::from_home(&home_dir);
        clear_cache(&layout).expect("a missing cache directory is not an error");
        let _ = fs::remove_dir_all(&home_dir);
    }
}
