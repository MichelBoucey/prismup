use crate::internals::network::expected_sha256_from_digest;
use crate::internals::platform::Platform;
use crate::types::Release;
use colored::Colorize;
use regex::Regex;
use semver::Version;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

pub fn get_versions_list(
    released_versions: &[Version],
    available_versions: &[Version],
    installed_versions: &[Version],
    current_version: Option<&Version>,
    platform: &Platform,
) {
    for version in released_versions.iter().rev() {
        let status = match installed_versions.contains(version) {
            true if current_version == Some(version) => {
                Some("(installed, current)".green().to_string())
            }
            true => Some("(installed)".green().to_string()),
            false if !available_versions.contains(version) => Some(
                format!("(no binary for {})", platform.target())
                    .yellow()
                    .to_string(),
            ),
            false => None,
        };
        match status {
            Some(status) => println!(
                "{} {} {}",
                "Prism".bold(),
                version.to_string().bold(),
                status
            ),
            None => println!("{} {}", "Prism".dimmed(), version.to_string().dimmed()),
        }
    }
}

pub fn get_released_prism_versions(releases: &[Release]) -> Vec<Version> {
    let mut versions: Vec<Version> = releases
        .iter()
        .filter(|release| !release.draft && !release.prerelease)
        .filter_map(|release| to_semver(&release.tag_name).ok())
        .collect();
    versions.sort();
    versions.dedup();
    versions
}

pub fn get_available_prism_versions(releases: &[Release], platform: &Platform) -> Vec<Version> {
    let mut versions: Vec<Version> = releases
        .iter()
        .filter(|release| !release.draft && !release.prerelease)
        .filter_map(|release| {
            let version = to_semver(&release.tag_name).ok()?;
            has_asset(release, &platform.archive_filename(&version)).then_some(version)
        })
        .collect();
    versions.sort();
    versions.dedup();
    versions
}

pub fn get_prism_installed_versions(path: &Path) -> Option<Vec<Version>> {
    let mut releases: Vec<Version> = fs::read_dir(path)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| to_semver(&entry.file_name().to_string_lossy()).ok())
        .collect();
    if releases.is_empty() {
        None
    } else {
        releases.sort();
        Some(releases)
    }
}

pub fn expected_sha256_for(
    releases: &[Release],
    platform: &Platform,
    version: &Version,
) -> Option<String> {
    let archive_filename = platform.archive_filename(version);
    releases
        .iter()
        .find_map(|release| {
            release
                .assets
                .iter()
                .find(|asset| asset.name == archive_filename)
        })
        .and_then(|asset| expected_sha256_from_digest(&asset.digest))
}

#[cfg(test)]
pub fn fixture_releases() -> Vec<Release> {
    serde_json::from_str(include_str!("tests/releases.json"))
        .expect("the releases fixture should be deserialized")
}

fn has_asset(release: &Release, asset_name: &str) -> bool {
    release.assets.iter().any(|asset| asset.name == asset_name)
}

pub fn to_semver(string: &str) -> Result<Version, String> {
    let semver_str = get_semver(string)?;
    semver_str
        .parse()
        .map_err(|e: semver::Error| format!("Invalid semver '{}': {}", string, e))
}

pub fn get_semver(s: &str) -> Result<String, String> {
    static VERSION_REGEX: OnceLock<Regex> = OnceLock::new();
    let regex = VERSION_REGEX.get_or_init(|| Regex::new(r"(\d+\.\d+\.\d+)").unwrap());
    regex
        .captures(s)
        .map(|c| c[1].to_string())
        .ok_or_else(|| format!("No version found in '{}'.", s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::internals::paths::{Layout, temp_home, temp_layout};
    use std::fs;

    fn release_named(releases: &[Release], tag_name: &str) -> Release {
        releases
            .iter()
            .find(|release| release.tag_name == tag_name)
            .cloned()
            .unwrap_or_else(|| panic!("the release '{}' should be in the fixture", tag_name))
    }

    fn linux_platform() -> Platform {
        Platform::from_uname("Linux", "x86_64").expect("Linux on x86_64 is supported")
    }

    fn mac_platform() -> Platform {
        Platform::from_uname("Darwin", "arm64").expect("macOS on arm64 is supported")
    }

    #[test]
    fn to_semver_reads_plain_and_prefixed_versions() {
        assert_eq!(
            to_semver("0.22.0").expect("a plain version"),
            Version::new(0, 22, 0)
        );
        assert_eq!(
            to_semver("v0.22.0").expect("a prefixed version"),
            Version::new(0, 22, 0)
        );
        assert_eq!(
            to_semver("/home/account/.prismup/prism/0.22.0/prism").expect("a path"),
            Version::new(0, 22, 0)
        );
    }

    #[test]
    fn to_semver_rejects_strings_without_version() {
        assert!(to_semver("").is_err());
        assert!(to_semver("0.3").is_err());
        assert!(to_semver("latest").is_err());
    }

    #[test]
    fn get_released_prism_versions_sorts_and_skips_drafts_and_prereleases() {
        let releases = fixture_releases();
        let mut draft = release_named(&releases, "v0.22.0");
        draft.tag_name = "v0.23.0".to_string();
        draft.draft = true;
        let mut prerelease = release_named(&releases, "v0.22.0");
        prerelease.tag_name = "v0.24.0-rc1".to_string();
        prerelease.prerelease = true;
        let mut all = releases;
        all.append(&mut vec![draft.clone(), prerelease, draft]);
        assert_eq!(
            get_released_prism_versions(&all),
            vec![
                Version::new(0, 1, 0),
                Version::new(0, 2, 0),
                Version::new(0, 3, 0),
                Version::new(0, 22, 0)
            ]
        );
    }

    #[test]
    fn get_available_prism_versions_keeps_only_the_platform_binaries() {
        let releases = fixture_releases();
        assert_eq!(
            get_available_prism_versions(&releases, &linux_platform()),
            vec![Version::new(0, 22, 0)]
        );
        assert_eq!(
            get_available_prism_versions(&releases, &mac_platform()),
            vec![Version::new(0, 3, 0)]
        );
    }

    #[test]
    fn expected_sha256_for_reads_the_asset_digest() {
        let releases = fixture_releases();
        assert_eq!(
            expected_sha256_for(&releases, &linux_platform(), &Version::new(0, 22, 0)).as_deref(),
            Some("sha256:39e84f0677d722cf15e8142f016f1b39a55640c6d14b5f8120dfbadb5ffa807a")
        );
        assert_eq!(
            expected_sha256_for(&releases, &mac_platform(), &Version::new(0, 3, 0)).as_deref(),
            Some("sha256:e6eaed5418d6cdc7c16aa03f1c4ca8d156d1bfb5b6ef3379962f606c5461578e")
        );
    }

    #[test]
    fn expected_sha256_for_is_none_for_a_version_without_asset() {
        let releases = fixture_releases();
        assert_eq!(
            expected_sha256_for(&releases, &linux_platform(), &Version::new(0, 3, 0)),
            None
        );
        assert_eq!(
            expected_sha256_for(&releases, &mac_platform(), &Version::new(0, 2, 0)),
            None
        );
    }

    #[test]
    fn get_prism_installed_versions_reads_and_sorts_the_versions_directory() {
        let layout = temp_layout("installed-versions");
        for version in ["0.16.0", "0.22.0", "0.9.0"] {
            fs::create_dir_all(layout.versions_dir().join(version))
                .expect("a version directory should be created");
        }
        fs::write(layout.versions_dir().join("not-a-version"), "")
            .expect("a junk file should be created");
        let versions = get_prism_installed_versions(layout.versions_dir())
            .expect("some versions are installed");
        assert_eq!(
            versions,
            vec![
                Version::new(0, 9, 0),
                Version::new(0, 16, 0),
                Version::new(0, 22, 0)
            ]
        );
        let _ = fs::remove_dir_all(layout.root().parent().expect("a home directory"));
    }

    #[test]
    fn get_prism_installed_versions_is_none_without_any_version() {
        let layout = temp_layout("no-installed-version");
        assert_eq!(get_prism_installed_versions(layout.versions_dir()), None);
        assert_eq!(
            get_prism_installed_versions(&layout.root().join("unknown")),
            None
        );
        let _ = fs::remove_dir_all(layout.root().parent().expect("a home directory"));
    }

    #[test]
    fn get_prism_installed_versions_ignores_the_home_directory_name() {
        let home_dir = temp_home("version-in-home");
        let layout = Layout::from_home(&home_dir);
        fs::create_dir_all(layout.versions_dir().join("0.22.0"))
            .expect("a version directory should be created");
        assert_eq!(
            get_prism_installed_versions(layout.versions_dir()),
            Some(vec![Version::new(0, 22, 0)])
        );
        let _ = fs::remove_dir_all(&home_dir);
    }
}
