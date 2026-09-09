use colored::Colorize;
use flate2::read::GzDecoder;
use semver::Version;
use sha256::try_digest;
use std::fs::{File, exists, read_dir, read_link, remove_dir_all, remove_file, rename};
use std::os::unix::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use tar::Archive;
use tokio::fs::create_dir_all;
pub mod args;
pub mod network;
pub mod versions;
use crate::get_available_prism_versions;
use crate::get_prism_installed_versions;
use crate::internals::network::*;
use crate::to_semver;

pub fn prism_version_remove(
    prismup_root_dir: &str,
    available_versions: &[Version],
    installed_versions: &[Version],
    version: &Version,
) -> Result<(), Box<dyn std::error::Error>> {
    if available_versions.contains(version) {
        if installed_versions.contains(version) {
            match get_current_prism_version(prismup_root_dir) {
                Some(current_version) if current_version == *version => {
                    println!(
                        "Prism version {} is your current Prism compiler.",
                        version.to_string().bold()
                    );
                    println!("Please set another Prism version before removing this one.");
                }
                _ => {
                    remove_file(prismup_root_dir.to_owned() + "bin/prism-" + &version.to_string())?;
                    remove_dir_all(prismup_root_dir.to_owned() + "prism/" + &version.to_string())?;
                    println!("Prism version {} removed.", version.to_string().bold());
                }
            }
        } else {
            println!(
                "{} {}",
                version.to_string().red(),
                "is not an installed version of Prism.".red()
            );
        }
    } else {
        println!(
            "{} {}",
            version.to_string().red(),
            "is not even an available version of Prism.".red()
        );
    }

    Ok(())
}

pub fn get_current_prism_version(prismup_root_dir: &str) -> Option<Version> {
    let current_prism_target = read_link(prismup_root_dir.to_owned() + "bin/prism");
    if let Ok(target) = current_prism_target {
        to_semver(target.to_str()?).ok()
    } else {
        None
    }
}

pub fn set_current_prism_version(
    prismup_root_dir: &str,
    version: &Version,
) -> Result<(), Box<dyn std::error::Error>> {
    if get_current_prism_version(prismup_root_dir).as_ref() == Some(version) {
        println!(
            "Prism version {} is already set as your current Prism compiler.",
            version.to_string().bold()
        );
    } else {
        let prism_symlink = format!("{}bin/prism", prismup_root_dir);
        let target = format!("{}prism/{}/prism", prismup_root_dir, version);
        let path = Path::new(&target);
        if path.exists() {
            if exists(&prism_symlink)? {
                remove_file(&prism_symlink)?;
            }
            symlink(target, prism_symlink)?;
            println!(
                "Set Prism version {} as your current Prism compiler.",
                version.to_string().bold()
            );
        }
    }
    Ok(())
}

pub fn clear_cache(home_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let cache_dir = format!("{}.cache/prismup/", home_dir);
    for entry in read_dir(&cache_dir)? {
        let entry = entry?;
        remove_file(entry.path())?;
    }
    println!("{}", "PrismUp cache cleared.".green());
    Ok(())
}

pub async fn make_prismup_directories(home_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    create_dir_all(format!("{}.cache/prismup/", home_dir)).await?;
    create_dir_all(format!("{}.prismup/bin/", home_dir)).await?;
    create_dir_all(format!("{}.prismup/prism/", home_dir)).await?;
    Ok(())
}

pub async fn install_prism_upgrade(
    client: &reqwest::Client,
    architecture: &str,
    os: &str,
    home_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let available_prism_versions = get_available_prism_versions(client, home_dir).await?;
    let prism_installed_version =
        get_prism_installed_versions(&(home_dir.to_owned() + ".prismup/prism/"));
    let latest_prism_version_to_install = available_prism_versions
        .last()
        .ok_or(format!("{}", "No Prism version released yet.".red()))?;
    let prismup_root_dir = home_dir.to_owned() + ".prismup/";
    let is_latest_installed = prism_installed_version
        .as_ref()
        .is_some_and(|versions| versions.contains(latest_prism_version_to_install));
    if !is_latest_installed {
        println!(
            "Installation of the latest Prism compiler ({}).",
            latest_prism_version_to_install.to_string().bold()
        );
        install_prism_version(
            client,
            architecture,
            os,
            latest_prism_version_to_install,
            home_dir,
        )
        .await?;
    } else {
        println!(
            "The latest Prism version {} is already installed.",
            latest_prism_version_to_install.to_string().bold()
        );
    }
    set_current_prism_version(&prismup_root_dir, latest_prism_version_to_install)?;
    Ok(())
}

pub async fn install_prism_version(
    client: &reqwest::Client,
    architecture: &str,
    os: &str,
    version: &Version,
    home_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let download_dir = format!("{}.cache/prismup/", home_dir);
    let install_dir = format!("{}.prismup/prism/", home_dir);

    let architecture = if architecture == "arm64" {
        "aarch64"
    } else {
        architecture
    };

    let archive_os_string = match os {
        "Linux" => "unknown-linux-gnu".to_string(),
        "Darwin" => "apple-darwin".to_string(),
        &_ => {
            return Err(format!("{} {}", os.red(), "is not supported".red()).into());
        }
    };

    let version = version.to_string();
    let archive_filename = format!(
        "prism-{}-{}-{}.tar.gz",
        version, architecture, archive_os_string
    );
    let archive_url = format!(
        "https://github.com/sdiehl/prism/releases/download/v{}/{}",
        version, archive_filename
    );
    let download_filepath = download_dir + &archive_filename;

    download_backoff(client, &archive_url, &download_filepath).await?;

    let right_archive_sha256 = get_sha256(client, &(archive_url.clone() + ".sha256")).await?;

    if is_file_integrity_ok(&right_archive_sha256, Path::new(&download_filepath))? {
        let tar_gz = File::open(download_filepath)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive.unpack(install_dir.clone())?;

        let extracted_dir = install_dir.clone() + &archive_filename.replace(".tar.gz", "");
        let install_version_dir = install_dir + &version;
        if Path::new(&install_version_dir).exists() {
            remove_dir_all(&install_version_dir)?;
        }
        rename(&extracted_dir, &install_version_dir)?;

        fs::symlink(
            install_version_dir + "/prism",
            format!("{}.prismup/bin/prism-{}", home_dir, version),
        )?;
    } else {
        return Err(format!(
            "{}{}{}",
            "SHA256 integrity check failed for '".red(),
            archive_filename.red(),
            "'".red()
        )
        .into());
    }

    Ok(())
}

pub fn is_file_integrity_ok(right_sha256_hash: &str, filepath: &Path) -> Result<bool, String> {
    let file_sha256_hash = try_digest(filepath).map_err(|e| {
        format!(
            "{}{}{}: {}",
            "Failed to compute SHA256 for '".red(),
            filepath.display().to_string().red(),
            "': ".red(),
            e.to_string().red()
        )
    })?;
    Ok(("sha256:".to_owned() + &file_sha256_hash) == right_sha256_hash)
}
