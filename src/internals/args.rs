use clap::{Arg, ArgAction, Command};

pub fn cli() -> Command {
    Command::new("prism")
        .author("Michel Boucey, michel.boucey@gmail.com")
        .about("An installer/updater for the Prism language")
        .arg_required_else_help(false)
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
                .long("install")
                .required(false)
                .action(clap::ArgAction::Set)
                .value_name("SEMVER")
                .help("Install Prism in the given version"),
        )
        .arg(
            Arg::new("set")
                .short('s')
                .long("set")
                .required(false)
                .action(clap::ArgAction::Set)
                .value_name("SEMVER")
                .help("Set the current Prism to the given version"),
        )
        .arg(
            Arg::new("remove")
                .short('r')
                .long("remove-version")
                .required(false)
                .action(clap::ArgAction::Set)
                .value_name("SEMVER")
                .help("Remove the given Prism version"),
        )
}
