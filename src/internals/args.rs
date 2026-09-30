use clap::{Arg, ArgAction, ArgGroup, Command};

pub fn cli() -> Command {
    Command::new("prismup")
        .author("Michel Boucey, michel.boucey@gmail.com")
        .about("A CLI tool to install and manage versions of the Prism language.")
        .arg_required_else_help(false)
        .group(
            ArgGroup::new("actions")
                .args([
                    "version",
                    "currentversion",
                    "prismupgrade",
                    "versionslist",
                    "install",
                    "set",
                    "remove",
                    "cacheclear",
                    "uninstall",
                ])
                .multiple(false),
        )
        .group(
            ArgGroup::new("network")
                .args(["refresh", "offline"])
                .multiple(false),
        )
        .arg(
            Arg::new("version")
                .action(ArgAction::SetTrue)
                .short('v')
                .long("version")
                .help("Print PrismUp version"),
        )
        .arg(
            Arg::new("currentversion")
                .action(ArgAction::SetTrue)
                .short('c')
                .long("current-version")
                .help("Print the current Prism version"),
        )
        .arg(
            Arg::new("prismupgrade")
                .action(ArgAction::SetTrue)
                .short('u')
                .long("upgrade")
                .help("Install and set the latest Prism version"),
        )
        .arg(
            Arg::new("versionslist")
                .action(ArgAction::SetTrue)
                .short('l')
                .long("versions-list")
                .help("Show list of available Prism versions"),
        )
        .arg(
            Arg::new("install")
                .short('i')
                .long("install-version")
                .required(false)
                .action(ArgAction::Set)
                .value_name("SEMVER")
                .help("Install Prism in the given version"),
        )
        .arg(
            Arg::new("set")
                .short('s')
                .long("set-version")
                .required(false)
                .action(ArgAction::Set)
                .value_name("SEMVER")
                .help("Set the current Prism to the given version"),
        )
        .arg(
            Arg::new("remove")
                .short('r')
                .long("remove-version")
                .required(false)
                .action(ArgAction::Set)
                .value_name("SEMVER")
                .help("Remove the given Prism version"),
        )
        .arg(
            Arg::new("refresh")
                .action(ArgAction::SetTrue)
                .short('f')
                .long("refresh")
                .help("Ignore the cached Prism releases and get them again from Github"),
        )
        .arg(
            Arg::new("offline")
                .action(ArgAction::SetTrue)
                .short('o')
                .long("offline")
                .help("Only use the cached Prism releases and archives, without network access"),
        )
        .arg(
            Arg::new("cacheclear")
                .action(ArgAction::SetTrue)
                .short('C')
                .long("cache-clear")
                .help("Clear the PrismUp cache directory contents"),
        )
        .arg(
            Arg::new("uninstall")
                .action(ArgAction::SetTrue)
                .long("uninstall-prismup")
                .help("Uninstall PrismUp"),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_action_can_be_given_alone() {
        for action in [
            vec!["prismup", "-c"],
            vec!["prismup", "--versions-list"],
            vec!["prismup", "-i", "0.22.0"],
            vec!["prismup", "-r", "0.22.0"],
            vec!["prismup", "--cache-clear"],
            vec!["prismup", "--uninstall-prismup"],
        ] {
            assert!(
                cli().try_get_matches_from(action.clone()).is_ok(),
                "'{}' should be a valid command line",
                action.join(" ")
            );
        }
    }

    #[test]
    fn conflicting_actions_are_rejected() {
        for actions in [
            vec!["prismup", "-u", "-c"],
            vec!["prismup", "-c", "-l"],
            vec!["prismup", "-i", "0.22.0", "-s", "0.21.0"],
            vec!["prismup", "--cache-clear", "-u"],
            vec!["prismup", "--uninstall-prismup", "--cache-clear"],
        ] {
            assert!(
                cli().try_get_matches_from(actions.clone()).is_err(),
                "'{}' should be rejected",
                actions.join(" ")
            );
        }
    }

    #[test]
    fn network_options_are_read() {
        let matches = cli()
            .try_get_matches_from(["prismup", "--versions-list", "--offline"])
            .expect("'--versions-list --offline' is a valid command line");
        assert!(matches.get_flag("versionslist"));
        assert!(matches.get_flag("offline"));
        assert!(!matches.get_flag("refresh"));

        let matches = cli()
            .try_get_matches_from(["prismup", "-f"])
            .expect("'-f' is a valid command line");
        assert!(matches.get_flag("refresh"));

        let matches = cli()
            .try_get_matches_from(["prismup", "-o"])
            .expect("'-o' is a valid command line");
        assert!(matches.get_flag("offline"));
        assert!(!matches.get_flag("refresh"));
    }

    #[test]
    fn refresh_and_offline_are_mutually_exclusive() {
        assert!(
            cli()
                .try_get_matches_from(["prismup", "--refresh", "--offline"])
                .is_err()
        );
        assert!(cli().try_get_matches_from(["prismup", "-f", "-o"]).is_err());
    }

    #[test]
    fn no_action_is_allowed() {
        assert!(cli().try_get_matches_from(["prismup"]).is_ok());
    }
}
