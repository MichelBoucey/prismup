use crate::internals::network::*;
use flate2::read::GzDecoder;
use semver::Version;
use sha256::try_digest;
use std::fs::{File, read_link, remove_dir_all, remove_file, rename};
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
use crate::to_semver;
use std::fs::exists;

pub fn prism_version_remove(
    prismup_root_dir: &str,
    version: &Version,
) -> Result<(), Box<dyn std::error::Error>> {
    let current_prism_version = get_current_prism_version(prismup_root_dir);
    if *version == *current_prism_version.as_ref().unwrap() {
        println!("Prism version {} is your current Prism compiler.", version);
        println!("Please set another Prism version before removing this one.");
    } else {
        remove_file(prismup_root_dir.to_owned() + "bin/prism-" + &version.to_string())?;
        remove_dir_all(prismup_root_dir.to_owned() + "prism/" + &version.to_string())?;
        println!("Prism version {} removed.", version);
    }
    Ok(())
}

pub fn get_current_prism_version(prismup_root_dir: &str) -> Option<Version> {
    let current_prism_target = read_link(prismup_root_dir.to_owned() + "bin/prism");
    if let Ok(target) = current_prism_target {
        Some(to_semver(target.to_str()?).unwrap())
    } else {
        None
    }
}

pub fn set_current_prism_version(
    prismup_root_dir: &str,
    version: &Version,
) -> Result<(), Box<dyn std::error::Error>> {
    let current_prism_version = get_current_prism_version(prismup_root_dir);
    if current_prism_version.is_some() && current_prism_version.unwrap() == *version {
        println!(
            "Prism version {} is already set as your current Prism compiler.",
            version
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
                version
            );
        }
    }
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
    let latest_prism_version_to_install = available_prism_versions.last().unwrap();
    if prism_installed_version.is_none()
        || !prism_installed_version
            .unwrap()
            .contains(latest_prism_version_to_install)
    {
        println!(
            "Installation of the latest Prism compiler ({}).",
            latest_prism_version_to_install
        );
        install_prism_version(
            client,
            architecture,
            os,
            latest_prism_version_to_install,
            home_dir,
        )
        .await?;
        set_current_prism_version(
            &(home_dir.to_owned() + ".prismup/"),
            latest_prism_version_to_install,
        )?;
    } else {
        println!(
            "The latest Prism version {} is already installed.",
            latest_prism_version_to_install
        );
        set_current_prism_version(
            &(home_dir.to_owned() + ".prismup/"),
            latest_prism_version_to_install,
        )?;
    }
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
        &_ => todo!(), // TODO!
    };

    let version = &format!("{}", version).to_string();
    let archive_filename =
        "prism-".to_owned() + version + "-" + architecture + "-" + &archive_os_string + ".tar.gz";
    let archive_url = "https://github.com/sdiehl/prism/releases/download/v".to_string()
        + version
        + "/"
        + &archive_filename;
    let download_filepath = download_dir + &archive_filename;

    download(client, &archive_url, &download_filepath).await?;

    let right_archive_sha256 = get_sha256(client, &(archive_url.clone() + ".sha256")).await;

    if is_file_integrity_ok(
        &right_archive_sha256.unwrap(),
        Path::new(&download_filepath),
    )? {
        let tar_gz = File::open(download_filepath)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive.unpack(install_dir.clone())?;

        rename(
            install_dir.clone()
                + "prism-"
                + version
                + "-"
                + architecture
                + "-"
                + &archive_os_string,
            install_dir.clone() + version,
        )?;

        fs::symlink(
            install_dir.clone() + version + "/prism",
            format!("{}.prismup/bin/prism-{}", home_dir, version),
        )?;
    }

    Ok(())
}

pub fn is_file_integrity_ok(right_sha256_hash: &str, filepath: &Path) -> Result<bool, String> {
    let file_sha256_hash = try_digest(filepath).map_err(|e| {
        format!(
            "Failed to compute SHA256 for '{}': {}",
            filepath.display(),
            e
        )
    })?;
    Ok(("sha256:".to_owned() + &file_sha256_hash) == right_sha256_hash)
}
