mod internals;
mod types;
use crate::internals::args::*;
use crate::internals::network::web_client;
use crate::internals::*;
use crate::types::*;
use crate::versions::*;
use std::env;
use std::error::Error;
use std::process::Command;
use std::process::exit;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let home_dir = env::var("HOME")? + "/";
    let prismup_root_dir: &str = &(home_dir.clone() + ".prismup/");

    if fs::metadata(prismup_root_dir).await.is_err() {
        make_prismup_directories(&home_dir).await?;
    };

    let matches = cli().get_matches();

    let flag_version = matches.get_one::<bool>("version").expect("required");
    if *flag_version {
        println!(
            "{}",
            "prismup ".to_owned()
                + env!("CARGO_PKG_VERSION")
                + " ("
                + env!("GIT_COMMIT_SHORT_HASH")
                + ") released under 3-Clause BSD License"
        );
        println!("Copyright © 2026 Michel Boucey (michel.boucey@gmail.com)");
        exit(0)
    }

    let prism_version_to_install = matches.get_one::<String>("install").map(|s| to_semver(s));
    let prism_version_to_set = matches.get_one::<String>("set").map(|s| to_semver(s));
    let prism_version_to_remove = matches.get_one::<String>("remove").map(|s| to_semver(s));
    let prism_version_upgrade = matches.get_one::<bool>("prismupgrade");
    let print_current_prism_version = matches.get_one::<bool>("currentversion");
    let print_prism_versions_list = matches.get_one::<bool>("versionslist");

    if print_current_prism_version == Some(&true) {
        match get_current_prism_version(prismup_root_dir) {
            Some(version) => println!("Prism {}", version),
            None => println!("No Prism version set"),
        }
        return Ok(());
    }

    let uname_m = Command::new("uname")
        .arg("-m")
        .output()
        .map_err(|e| format!("Failed to run 'uname -m': {}", e))?;
    let architecture = String::from_utf8_lossy(&uname_m.stdout)
        .trim_end()
        .to_string();

    let uname = Command::new("uname")
        .output()
        .map_err(|e| format!("Failed to run 'uname': {}", e))?;
    let os = String::from_utf8_lossy(&uname.stdout)
        .trim_end()
        .to_string();

    let web_client = web_client().await;

    let prism_installed_versions =
        get_prism_installed_versions(&(prismup_root_dir.to_owned() + "prism/"));

    if print_prism_versions_list == Some(&true) {
        let available_prism_versions = get_available_prism_versions(&web_client, &home_dir).await?;
        get_versions_list(
            &available_prism_versions,
            &prism_installed_versions.unwrap(),
        );
        return Ok(());
    }

    if prism_installed_versions.is_none() {
        println!("No Prism compiler installed yet.");
        install_prism_upgrade(&web_client, &architecture, &os, &home_dir).await?;
        println!("Please add '$HOME/.prismup/bin/' to your PATH.");
        return Ok(());
    }

    if prism_version_upgrade == Some(&true) {
        install_prism_upgrade(&web_client, &architecture, &os, &home_dir).await?;
        return Ok(());
    }
    if let Some(Ok(version)) = prism_version_to_install {
        if prism_installed_versions.is_none()
            || !prism_installed_versions
                .as_ref()
                .unwrap()
                .contains(&version)
        {
            let current_prism_versions =
                get_available_prism_versions(&web_client, &home_dir).await?;
            if current_prism_versions.contains(&version) {
                println!("Installation of Prism compiler version {}...", version);
                install_prism_version(&web_client, &architecture, &os, &version, &home_dir).await?;
            } else {
                println!("{} is not a Prism version released.", version);
            }
        } else {
            println!("Prism {} already installed.", version);
        }
        return Ok(());
    }

    if let Some(Ok(version)) = prism_version_to_set {
        if prism_installed_versions.is_some()
            && prism_installed_versions
                .as_ref()
                .unwrap()
                .contains(&version)
        {
            let _ = set_current_prism_version(prismup_root_dir, &version);
        } else {
            let current_prism_versions =
                get_available_prism_versions(&web_client, &home_dir).await?;
            if current_prism_versions.contains(&version) {
                println!(
                    "Prism version {} needs to be installed before being set.",
                    version
                );
                install_prism_version(&web_client, &architecture, &os, &version, &home_dir).await?;
            } else {
                println!("Sorry but {} is not a Prism version released.", version);
            }
            let _ = set_current_prism_version(prismup_root_dir, &version);
        }
        return Ok(());
    }

    if let Some(Ok(version)) = prism_version_to_remove
        && prism_installed_versions.is_some()
        && prism_installed_versions.unwrap().contains(&version)
    {
        let _ = prism_version_remove(prismup_root_dir, &version);
    }

    Ok(())
}
