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
            parse(&args(&["convert", "libro.kfx-zip"])),
            Ok(Command::Convert { input: PathBuf::from("libro.kfx-zip"), out_dir: None })
        );
    }

    #[test]
    fn output_folder_can_go_before_or_after_the_input() {
        let expected = Ok(Command::Convert { input: PathBuf::from("a.kfx"), out_dir: Some(PathBuf::from("salida")) });
        assert_eq!(parse(&args(&["convert", "a.kfx", "-o", "salida"])), expected);
        assert_eq!(parse(&args(&["convert", "-o", "salida", "a.kfx"])), expected);
        assert_eq!(parse(&args(&["convert", "a.kfx", "--output", "salida"])), expected);
    }

    #[test]
    fn help_and_version() {
        assert_eq!(parse(&args(&["--help"])), Ok(Command::Help));
        assert_eq!(parse(&args(&["-h"])), Ok(Command::Help));
        assert_eq!(parse(&args(&["convert", "--help"])), Ok(Command::Help));
        assert_eq!(parse(&args(&["--version"])), Ok(Command::Version));
    }

    #[test]
    fn mistakes_are_explained() {
        for bad in [
            &["convert"][..],
            &["convert", "-o", "salida"],
            &["convert", "a.kfx", "-o"],
            &["convert", "a.kfx", "b.kfx"],
            &["convert", "a.kfx", "--nope"],
            &["explode"],
            &["a.kfx"],
        ] {
            let result = parse(&args(bad));
            assert!(matches!(&result, Err(message) if !message.is_empty()), "{bad:?} -> {result:?}");
        }
    }

    #[test]
    fn a_file_name_that_starts_with_a_dash_needs_no_special_syntax_after_the_flags() {
        assert_eq!(
            parse(&args(&["convert", "--", "-raro.kfx"])),
            Ok(Command::Convert { input: PathBuf::from("-raro.kfx"), out_dir: None })
        );
    }
}
