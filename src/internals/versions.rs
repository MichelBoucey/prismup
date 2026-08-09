use crate::internals::get_prism_releases;
use colored::Colorize;
use regex::Regex;
use semver::Version;
use std::fs;
use std::sync::OnceLock;

pub fn get_versions_list(available_versions: &[Version], versions_installed: &[Version]) {
    let mut versions = available_versions.to_vec();
    versions.reverse();
    for version in versions.iter() {
        if versions_installed.contains(version) {
            println!(
                "{} {} {}",
                "Prism".bold(),
                version.to_string().bold(),
                "(installed)".green()
            );
        } else {
            println!("{} {}", "Prism".dimmed(), version.to_string().dimmed());
        }
    }
}

pub fn get_prism_installed_versions(path: &str) -> Option<Vec<Version>> {
    let mut releases = Vec::new();
    let entries = fs::read_dir(path).ok()?;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        if let Ok(version) = to_semver(&file_name.to_string_lossy()) {
            releases.push(version);
        }
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
    let releases = get_prism_releases(client, home_dir)
        .await
        .map_err(|_| "Failed to get Prism releases from Github.".to_string())?;
    let mut versions = Vec::new();
    for release in releases.iter() {
        versions.push(to_semver(&release.tag_name)?);
    }
    versions.sort();
    Ok(versions)
}

pub fn to_semver(string: &str) -> Result<Version, String> {
    let semver_str = get_semver(string)?;
    semver_str
        .parse()
        .map_err(|e| format!("Invalid semver '{}': {}", string, e))
}

pub fn get_semver(s: &str) -> Result<String, String> {
    static VERSION_REGEX: OnceLock<Regex> = OnceLock::new();
    let regex = VERSION_REGEX.get_or_init(|| Regex::new(r"(\d+\.\d+\.\d+)").unwrap());
    regex
        .captures(s)
        .map(|c| c[1].to_string())
        .ok_or_else(|| format!("No version found in '{}'", s))
}
