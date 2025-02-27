use std::env;
use std::io;
use std::io::prelude::*;
use std::path::Path;

use lscolors::{LsColors, Style};

#[cfg(all(
    not(feature = "nu-ansi-term"),
    not(feature = "gnu_legacy"),
    not(feature = "ansi_term"),
    not(feature = "crossterm"),
    not(feature = "owo-colors")
))]
compile_error!(
    "one feature must be enabled: ansi_term, nu-ansi-term, crossterm, gnu_legacy, owo-colors"
);

fn print_path(handle: &mut dyn Write, ls_colors: &LsColors, path: &str, is_dir: bool) -> io::Result<()> {
    for (component, style) in ls_colors.style_for_path_components(Path::new(path)) {
        #[cfg(any(feature = "nu-ansi-term", feature = "gnu_legacy"))]
        {
            let ansi_style = style.map(Style::to_nu_ansi_term_style).unwrap_or_default();
            write!(handle, "{}", ansi_style.paint(component.to_string_lossy()))?;
        }

        #[cfg(feature = "ansi_term")]
        {
            let ansi_style = style.map(Style::to_ansi_term_style).unwrap_or_default();
            write!(handle, "{}", ansi_style.paint(component.to_string_lossy()))?;
        }

        #[cfg(feature = "crossterm")]
        {
            let ansi_style = style.map(Style::to_crossterm_style).unwrap_or_default();
            write!(handle, "{}", ansi_style.apply(component.to_string_lossy()))?;
        }
        #[cfg(feature = "owo-colors")]
        {
            use owo_colors::OwoColorize;
            let ansi_style = style.map(Style::to_owo_colors_style).unwrap_or_default();
            write!(handle, "{}", component.to_string_lossy().style(ansi_style))?;
        }
    }
    if is_dir && !path.eq("/") {
        write!(handle, "/")?;
    }
    writeln!(handle)?;

    Ok(())
}

fn print_tilde_path(handle: &mut dyn Write, ls_colors: &LsColors, path: &str, home_slash_count: usize, is_dir: bool) -> io::Result<()> {
    let mut i = 0;
    for (component, style) in ls_colors.style_for_path_components(Path::new(path)) {
        i = i + 1;
        let mut comp_str = component.to_string_lossy();
        if i < home_slash_count {
            // ignore the starting components of $HOME (e.g. "/" "home/" )
            continue;
        } else if i == home_slash_count {
            // treat the last component of $HOME as "~/" (e.g. "user/" )
            comp_str = "~/".to_string().into();
        }
        #[cfg(any(feature = "nu-ansi-term", feature = "gnu_legacy"))]
        {
            let ansi_style = style.map(Style::to_nu_ansi_term_style).unwrap_or_default();
            write!(handle, "{}", ansi_style.paint(comp_str))?;
        }

        #[cfg(feature = "ansi_term")]
        {
            let ansi_style = style.map(Style::to_ansi_term_style).unwrap_or_default();
            write!(handle, "{}", ansi_style.paint(comp_str))?;
        }

        #[cfg(feature = "crossterm")]
        {
            let ansi_style = style.map(Style::to_crossterm_style).unwrap_or_default();
            write!(handle, "{}", ansi_style.paint(comp_str))?;
        }
        #[cfg(feature = "owo-colors")]
        {
            use owo_colors::OwoColorize;
            let ansi_style = style.map(Style::to_owo_colors_style).unwrap_or_default();
            write!(handle, "{}", comp_str.style(ansi_style))?;
        }
    }
    if is_dir && !path.eq("/") {
        write!(handle, "/")?;
    }
    writeln!(handle)?;

    Ok(())
}

fn run() -> io::Result<()> {
    let ls_colors = LsColors::from_env().unwrap_or_default();
    let home_dir = env::var("HOME").unwrap_or_else(|_| "".to_string());
    let home_slash_count = home_dir.matches('/').count() + 1;

    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    let mut args = env::args();

    if args.len() >= 2 {
        // Skip program name
        args.next();

        for arg in args {
            if home_dir.len() > 0 {
                if let Some(pos) = arg.find("~/") {
                    if pos == 0 {
                        let path_conv_str = arg.replacen("~", &home_dir, 1);
                        let path = Path::new::<str>(path_conv_str.as_ref());
                        if !path.exists() {
                            writeln!(stdout, "\x1b[31m{}\x1b[0m", arg)?; // red line for the path-not-found case
                        } else {
                            print_tilde_path(&mut stdout, &ls_colors, path_conv_str.as_ref(), home_slash_count, path.is_dir())?;
                        }
                        continue;
                    }
                }
            }
            let path = Path::new(&arg);
            if !path.exists() {
                writeln!(stdout, "\x1b[31m{}\x1b[0m", arg)?; // red line for the path-not-found case
            } else {
                print_path(&mut stdout, &ls_colors, &arg, path.is_dir())?;
            }
        }
    } else {
        let stdin = io::stdin();
        let mut buf = vec![];

        while let Ok(size) = stdin.lock().read_until(b'\n', &mut buf) {
            if size == 0 {
                break;
            }

            let path_str = String::from_utf8_lossy(&buf[..(buf.len() - 1)]);
            #[cfg(windows)]
            let path_str = path_str.trim_end_matches('\r');

            if home_dir.len() > 0 {
                if let Some(pos) = path_str.find("~/") {
                    if pos == 0 {
                        let path_conv_str = path_str.replacen("~", &home_dir, 1);
                        let path = Path::new::<str>(path_conv_str.as_ref());
                        if !path.exists() {
                            writeln!(stdout, "\x1b[31m{}\x1b[0m", path_str)?; // red line for the path-not-found case
                        } else {
                            print_tilde_path(&mut stdout, &ls_colors, path_conv_str.as_ref(), home_slash_count, path.is_dir())?;
                        }
                        buf.clear();
                        continue;
                    }
                }
            }

            let path = Path::new(path_str.as_ref());
            if !path.exists() {
                writeln!(stdout, "\x1b[31m{}\x1b[0m", path_str)?; // red line for the path-not-found case
            } else {
                print_path(&mut stdout, &ls_colors, path_str.as_ref(), path.is_dir())?;
            }

            buf.clear();
        }
    }

    Ok(())
}

fn main() {
    run().ok();
}
