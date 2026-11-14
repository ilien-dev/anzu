//! Argumentos de la línea de comandos.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Sin argumentos: abre la interfaz.
    Gui,
    Convert { input: PathBuf, out_dir: Option<PathBuf> },
    Help,
    Version,
}

pub const USAGE: &str = "\
Anzu: de Kindle a EPUB

Uso:
  anzu                              Abre la interfaz
  anzu convert <archivo> [-o <carpeta>]
                                      Convierte un KFX sin DRM (.kfx-zip) a EPUB
  anzu --help | --version

Opciones de convert:
  -o, --output <carpeta>              Dónde guardar el EPUB (por defecto, la carpeta actual)";

pub fn parse(args: &[String]) -> Result<Command, String> {
    let Some((first, rest)) = args.split_first() else { return Ok(Command::Gui) };
    match first.as_str() {
        "-h" | "--help" => Ok(Command::Help),
        "-V" | "--version" => Ok(Command::Version),
        "convert" => parse_convert(rest),
        other => Err(format!("orden desconocida: {other}")),
    }
}

fn parse_convert(args: &[String]) -> Result<Command, String> {
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
                    let dir = rest.next().ok_or_else(|| format!("falta la carpeta después de {arg}"))?;
                    out_dir = Some(PathBuf::from(dir));
                    continue;
                }
                flag if flag.len() > 1 && flag.starts_with('-') => return Err(format!("opción desconocida: {flag}")),
                _ => {}
            }
        }
        if input.is_some() {
            return Err("solo se puede convertir un archivo cada vez".to_string());
        }
        input = Some(PathBuf::from(arg));
    }
    let input = input.ok_or_else(|| "falta el archivo que convertir".to_string())?;
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
