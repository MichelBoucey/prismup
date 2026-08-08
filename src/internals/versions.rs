use crate::VecVersion;
use crate::internals::get_prism_releases;
use regex::Regex;
use semver::Version;
use std::fs;

pub fn get_versions_list(available_versions: &[Version], versions_installed: &[Version]) {
    let mut versions = available_versions.to_vec();
    versions.reverse();
    for version in VecVersion(versions).0.iter() {
        print!("Prism {}", version);
        if versions_installed.contains(version) {
            println!(" (is installed)");
        } else {
            println!();
        }
    }
}

pub fn get_prism_installed_versions(path: &str) -> Option<Vec<Version>> {
    let mut releases = Vec::new();
    let entries = fs::read_dir(path).unwrap();
    for entry in entries {
        let version = to_semver(&entry.unwrap().file_name().to_string_lossy())
            .to_owned()
            .unwrap();
        let _ = &releases.push(version);
    }
    if releases.is_empty() {
        None
    } else {
        releases.sort();
        Some(releases)
    }
}

pub async fn get_available_prism_versions(
    client: &reqwest::Client,
    home_dir: &str,
) -> Result<Vec<Version>, String> {
    let mut versions = Vec::new();
    let releases = get_prism_releases(client, home_dir).await;
    if let Ok(releases) = releases {
        for release in releases.iter() {
            versions.push(to_semver(&release.tag_name).unwrap());
        }
        versions.sort();
        Ok(versions)
    } else {
        Err("Failed to get Prism releases from Github.".to_string())
    }
}

pub fn to_semver(string: &str) -> Result<Version, String> {
    let semver_str = get_semver(string)?;
    semver_str
        .parse()
        .map_err(|e| format!("Invalid semver '{}': {}", string, e))
}

pub fn get_semver(s: &str) -> Result<String, String> {
    let regex = Regex::new(r"(\d*\.\d*\.\d*)").unwrap();
    regex
        .captures(s)
        .map(|c| c[1].to_string())
        .ok_or_else(|| format!("No version found in '{}'", s))
}
