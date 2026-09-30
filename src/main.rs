mod internals;
mod types;

use crate::internals::args::*;
use crate::internals::network::{get_prism_releases, web_client};
use crate::internals::paths::Layout;
use crate::internals::platform::Platform;
use crate::internals::versions::to_semver;
use crate::internals::{
    PrismUp, clear_cache, get_current_prism_version, make_prismup_directories, uninstall,
};
use crate::types::NetworkOptions;
use colored::Colorize;
use std::env;
use std::path::Path;
use std::process::exit;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{}", error.red());
        exit(1);
    }
}

async fn run() -> Result<(), String> {
    let layout = prismup_layout()?;
    if !layout.root().exists() {
        make_prismup_directories(&layout).await?;
    };

    let matches = cli().get_matches();

    if matches.get_flag("cacheclear") {
        clear_cache(&layout)?;
        return Ok(());
    }

    if matches.get_flag("uninstall") {
        uninstall(&layout)?;
        return Ok(());
    }

    if matches.get_flag("version") {
        println!(
            "PrismUp {} ({}) released under 3-Clause BSD License",
            env!("CARGO_PKG_VERSION").to_string().bold(),
            env!("GIT_COMMIT_SHORT_HASH")
        );
        println!("Copyright © 2026 Michel Boucey (michel.boucey@gmail.com)");
        return Ok(());
    }

    if matches.get_flag("currentversion") {
        match get_current_prism_version(&layout) {
            Some(version) => println!("Prism {}", version.to_string().bold()),
            None => println!("No Prism version set"),
        }
        return Ok(());
    }

    let platform = Platform::current()?;
    let options = NetworkOptions {
        force_refresh: matches.get_flag("refresh"),
        offline: matches.get_flag("offline"),
    };
    let web_client = web_client().await?;
    let releases = get_prism_releases(&web_client, &layout, options).await?;
    let prismup = PrismUp::new(&web_client, &layout, &platform, &releases, options);

    if matches.get_flag("versionslist") {
        prismup.versions_list();
        return Ok(());
    }

    if prismup.installed_versions().is_none() {
        println!("No Prism compiler installed yet.");
        prismup.upgrade().await?;
        println!("Please add '$HOME/.prismup/bin/' to your PATH.");
        return Ok(());
    }

    if matches.get_flag("prismupgrade") {
        prismup.upgrade().await?;
        return Ok(());
    }

    if let Some(prism_version) = matches.get_one::<String>("install") {
        prismup.install(&to_semver(prism_version)?).await?;
        return Ok(());
    }

    if let Some(prism_version) = matches.get_one::<String>("set") {
        prismup.set(&to_semver(prism_version)?).await?;
        return Ok(());
    }

    if let Some(prism_version) = matches.get_one::<String>("remove") {
        prismup.remove(&to_semver(prism_version)?)?;
        return Ok(());
    }

    Ok(())
}

fn prismup_layout() -> Result<Layout, String> {
    let home_dir = env::var("HOME")
        .map_err(|e| format!("Failed to get the home directory from 'HOME': {}", e))?;
    Ok(Layout::from_home(Path::new(&home_dir)))
}
