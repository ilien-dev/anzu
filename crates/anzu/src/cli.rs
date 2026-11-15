//! Command-line arguments.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// No arguments: open the window.
    Gui,
    Convert { input: PathBuf, out_dir: Option<PathBuf> },
    Help,
    Version,
    /// Open the installer, whatever the name of the executable.
    Install,
    /// Replace the installation in `dir` with this executable once the app there has
    /// closed, then open it. The app runs this when the user restarts to update.
    Update { dir: PathBuf },
    /// Remove the installation. Windows runs this from the list of apps.
    Uninstall,
}

/// What was wrong with the arguments. The text for each lives in `i18n`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    UnknownCommand(String),
    UnknownOption(String),
    /// An option that needs a value came last.
    MissingValue(String),
    MissingInput,
    TooManyInputs,
}

pub fn parse(args: &[String]) -> Result<Command, CliError> {
    let Some((first, rest)) = args.split_first() else { return Ok(Command::Gui) };
    match first.as_str() {
        "-h" | "--help" => Ok(Command::Help),
        "-V" | "--version" => Ok(Command::Version),
        "convert" => parse_convert(rest),
        "--install" => Ok(Command::Install),
        "--uninstall" => Ok(Command::Uninstall),
        "--update" => match rest {
            [flag, dir] if flag == "--dir" => Ok(Command::Update { dir: PathBuf::from(dir) }),
            [flag] if flag == "--dir" => Err(CliError::MissingValue(flag.clone())),
            [] => Err(CliError::MissingValue("--dir".to_string())),
            [other, ..] => Err(CliError::UnknownOption(other.clone())),
        },
        other => Err(CliError::UnknownCommand(other.to_string())),
    }
}

fn parse_convert(args: &[String]) -> Result<Command, CliError> {
    let mut input: Option<PathBuf> = None;
    let mut out_dir = None;
    let mut only_files = false;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if !only_files {
            match arg.as_str() {
                "--" => {
                    only_files = true;
                    continue;
                }
                "-h" | "--help" => return Ok(Command::Help),
                "-o" | "--output" => {
                    let dir = rest.next().ok_or_else(|| CliError::MissingValue(arg.clone()))?;
                    out_dir = Some(PathBuf::from(dir));
                    continue;
                }
                flag if flag.len() > 1 && flag.starts_with('-') => return Err(CliError::UnknownOption(flag.to_string())),
                _ => {}
            }
        }
        if input.is_some() {
            return Err(CliError::TooManyInputs);
        }
        input = Some(PathBuf::from(arg));
    }
    let input = input.ok_or(CliError::MissingInput)?;
    Ok(Command::Convert { input, out_dir })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_arguments_opens_the_interface() {
        assert_eq!(parse(&args(&[])), Ok(Command::Gui));
    }

    #[test]
    fn convert_takes_an_input_file() {
        assert_eq!(
            parse(&args(&["convert", "book.kfx-zip"])),
            Ok(Command::Convert { input: PathBuf::from("book.kfx-zip"), out_dir: None })
        );
    }

    #[test]
    fn output_folder_can_go_before_or_after_the_input() {
        let expected = Ok(Command::Convert { input: PathBuf::from("a.kfx"), out_dir: Some(PathBuf::from("out")) });
        assert_eq!(parse(&args(&["convert", "a.kfx", "-o", "out"])), expected);
        assert_eq!(parse(&args(&["convert", "-o", "out", "a.kfx"])), expected);
        assert_eq!(parse(&args(&["convert", "a.kfx", "--output", "out"])), expected);
    }

    #[test]
    fn help_and_version() {
        assert_eq!(parse(&args(&["--help"])), Ok(Command::Help));
        assert_eq!(parse(&args(&["-h"])), Ok(Command::Help));
        assert_eq!(parse(&args(&["convert", "--help"])), Ok(Command::Help));
        assert_eq!(parse(&args(&["--version"])), Ok(Command::Version));
    }

    #[test]
    fn each_mistake_is_named() {
        assert_eq!(parse(&args(&["convert"])), Err(CliError::MissingInput));
        assert_eq!(parse(&args(&["convert", "-o", "out"])), Err(CliError::MissingInput));
        assert_eq!(parse(&args(&["convert", "a.kfx", "-o"])), Err(CliError::MissingValue("-o".into())));
        assert_eq!(parse(&args(&["convert", "a.kfx", "b.kfx"])), Err(CliError::TooManyInputs));
        assert_eq!(parse(&args(&["convert", "a.kfx", "--nope"])), Err(CliError::UnknownOption("--nope".into())));
        assert_eq!(parse(&args(&["explode"])), Err(CliError::UnknownCommand("explode".into())));
        assert_eq!(parse(&args(&["a.kfx"])), Err(CliError::UnknownCommand("a.kfx".into())));
    }

    #[test]
    fn install_update_and_uninstall() {
        assert_eq!(parse(&args(&["--install"])), Ok(Command::Install));
        assert_eq!(parse(&args(&["--uninstall"])), Ok(Command::Uninstall));
        assert_eq!(
            parse(&args(&["--update", "--dir", r"C:\Programs\Anzu"])),
            Ok(Command::Update { dir: PathBuf::from(r"C:\Programs\Anzu") })
        );
        assert_eq!(parse(&args(&["--update"])), Err(CliError::MissingValue("--dir".into())));
        assert_eq!(parse(&args(&["--update", "--dir"])), Err(CliError::MissingValue("--dir".into())));
        assert_eq!(parse(&args(&["--update", "--x"])), Err(CliError::UnknownOption("--x".into())));
    }

    #[test]
    fn a_file_name_that_starts_with_a_dash_goes_after_a_double_dash() {
        assert_eq!(
            parse(&args(&["convert", "--", "-odd.kfx"])),
            Ok(Command::Convert { input: PathBuf::from("-odd.kfx"), out_dir: None })
        );
    }
}
