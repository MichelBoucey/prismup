mod internals;
mod types;
use crate::internals::args::*;
use crate::internals::network::web_client;
use crate::internals::*;
use crate::versions::*;
use colored::Colorize;
use std::env;
use std::error::Error;
use std::process::{Command, exit};
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let home_dir = env::var("HOME")? + "/";
    let prismup_root_dir = format!("{home_dir}.prismup/");

    if fs::metadata(&prismup_root_dir).await.is_err() {
        make_prismup_directories(&home_dir).await?;
    };

    let matches = cli().get_matches();

    if matches.get_flag("version") {
        println!(
            "PrismUp {} ({}) released under 3-Clause BSD License",
            env!("CARGO_PKG_VERSION").to_string().bold(),
            env!("GIT_COMMIT_SHORT_HASH")
        );
        println!("Copyright © 2026 Michel Boucey (michel.boucey@gmail.com)");
        exit(0)
    }

    let prism_version_to_install = matches.get_one::<String>("install").map(|s| to_semver(s));
    let prism_version_to_set = matches.get_one::<String>("set").map(|s| to_semver(s));
    let prism_version_to_remove = matches.get_one::<String>("remove").map(|s| to_semver(s));
    let prism_version_upgrade = matches.get_flag("prismupgrade");
    let print_current_prism_version = matches.get_flag("currentversion");
    let print_prism_versions_list = matches.get_flag("versionslist");

    if print_current_prism_version {
        match get_current_prism_version(&prismup_root_dir) {
            Some(version) => println!("Prism {}", version.to_string().bold()),
            None => println!("No Prism version set"),
        }
        return Ok(());
    }

    let uname = Command::new("uname")
        .args(["-s", "-m"])
        .output()
        .map_err(|e| format!("Failed to run 'uname -sm': {}", e))?;
    let uname_output = String::from_utf8_lossy(&uname.stdout);
    let mut uname_fields = uname_output.split_whitespace();
    let os = match uname_fields.next() {
        Some(os) => os.to_string(),
        None => return Err("Failed to get the OS name from 'uname'.".into()),
    };
    let architecture = match uname_fields.next() {
        Some(architecture) => architecture.to_string(),
        None => return Err("Failed to get the architecture from 'uname'.".into()),
    };

    let web_client = web_client().await?;

    let prism_installed_versions =
        get_prism_installed_versions(&format!("{prismup_root_dir}prism/"));

    if print_prism_versions_list {
        let available_prism_versions = get_available_prism_versions(&web_client, &home_dir).await?;
        match &prism_installed_versions {
            Some(installed_versions) => {
                get_versions_list(&available_prism_versions, installed_versions)
            }
            None => get_versions_list(&available_prism_versions, &[]),
        }
        return Ok(());
    }

    if prism_installed_versions.is_none() {
        println!("No Prism compiler installed yet.");
        install_prism_upgrade(&web_client, &architecture, &os, &home_dir).await?;
        println!("Please add '$HOME/.prismup/bin/' to your PATH.");
        return Ok(());
    }

    if prism_version_upgrade {
        install_prism_upgrade(&web_client, &architecture, &os, &home_dir).await?;
        return Ok(());
    }

    if let Some(result) = prism_version_to_install {
        let version = match result {
            Ok(version) => version,
            Err(message) => {
                println!("{}", message);
                return Ok(());
            }
        };
        let is_installed = prism_installed_versions
            .as_ref()
            .is_some_and(|versions| versions.contains(&version));
        if is_installed {
            println!("Prism {} already installed.", version.to_string().bold());
        } else {
            let current_prism_versions =
                get_available_prism_versions(&web_client, &home_dir).await?;
            if current_prism_versions.contains(&version) {
                println!("Installation of Prism compiler version {}...", version);
                install_prism_version(&web_client, &architecture, &os, &version, &home_dir).await?;
            } else {
                println!(
                    "Sorry but {} is not a Prism version released.",
                    version.to_string().bold()
                );
            }
        }
        return Ok(());
    }

    if let Some(result) = prism_version_to_set {
        let version = match result {
            Ok(version) => version,
            Err(message) => {
                println!("{}", message);
                return Ok(());
            }
        };
        let is_installed = prism_installed_versions
            .as_ref()
            .is_some_and(|versions| versions.contains(&version));
        if is_installed {
            set_current_prism_version(&prismup_root_dir, &version)?;
        } else {
            let current_prism_versions =
                get_available_prism_versions(&web_client, &home_dir).await?;
            if current_prism_versions.contains(&version) {
                println!(
                    "Prism version {} needs to be installed before being set.",
                    version.to_string().bold()
                );
                install_prism_version(&web_client, &architecture, &os, &version, &home_dir).await?;
                set_current_prism_version(&prismup_root_dir, &version)?;
            } else {
                println!(
                    "Sorry but {} is not a Prism version released.",
                    version.to_string().bold()
                );
            }
        }
        return Ok(());
    }

    if let Some(result) = prism_version_to_remove {
        let version = match result {
            Ok(version) => version,
            Err(message) => {
                println!("{}", message);
                return Ok(());
            }
        };
        let is_installed = prism_installed_versions
            .as_ref()
            .is_some_and(|versions| versions.contains(&version));
        if is_installed {
            prism_version_remove(&prismup_root_dir, &version)?;
        }
        return Ok(());
    }

    Ok(())
}
