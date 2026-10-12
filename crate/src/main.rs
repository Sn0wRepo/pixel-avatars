use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use pixelid::{blockies, parse_address, pixel_id, Icon};

/// Deterministic pixel identicons from wallet addresses.
#[derive(Parser)]
#[command(version, about, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum)]
enum Style {
    /// ethereum-blockies (MetaMask's pixel identicon)
    Blockies,
    /// PixelID v1, identical to PixelAvatar.sol
    Pixelid,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Svg,
    Png,
}

#[derive(clap::Args)]
struct IconArgs {
    #[arg(short, long, value_enum, default_value_t = Style::Blockies)]
    style: Style,
    /// Grid size for Blockies (PixelID is always 8)
    #[arg(long, default_value_t = 8, value_parser = clap::value_parser!(u16).range(1..=64))]
    size: u16,
}

#[derive(Subcommand)]
enum Command {
    /// Print SVG markup to stdout
    Svg {
        /// Address (or any text seed for Blockies)
        seed: String,
        #[command(flatten)]
        icon: IconArgs,
    },
    /// Write a PNG at an integer scale
    Png {
        seed: String,
        #[command(flatten)]
        icon: IconArgs,
        /// Pixels per cell
        #[arg(long, default_value_t = 32)]
        scale: u16,
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Render every address in a file (one per line) into a directory
    Batch {
        input: PathBuf,
        #[arg(short, long, default_value = "avatars")]
        out_dir: PathBuf,
        #[command(flatten)]
        icon: IconArgs,
        #[arg(short, long, value_enum, default_value_t = Format::Svg)]
        format: Format,
        #[arg(long, default_value_t = 32)]
        scale: u16,
    },
}

fn make(seed: &str, args: &IconArgs) -> Result<Icon> {
    let seed = seed.trim();
    match args.style {
        Style::Pixelid => {
            let addr = parse_address(seed)
                .with_context(|| format!("PixelID needs a 0x address, got {seed:?}"))?;
            Ok(pixel_id(&addr))
        }
        Style::Blockies => {
            // Addresses are lowercased for MetaMask parity; other seeds are used verbatim.
            let s = if parse_address(seed).is_some() {
                seed.to_lowercase()
            } else {
                seed.to_string()
            };
            Ok(blockies(&s, usize::from(args.size)))
        }
    }
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Svg { seed, icon } => println!("{}", make(&seed, &icon)?.to_svg()),
        Command::Png {
            seed,
            icon,
            scale,
            out,
        } => {
            let png = make(&seed, &icon)?.to_png(usize::from(scale))?;
            fs::write(&out, png).with_context(|| format!("writing {}", out.display()))?;
            eprintln!("wrote {}", out.display());
        }
        Command::Batch {
            input,
            out_dir,
            icon,
            format,
            scale,
        } => {
            let text = fs::read_to_string(&input)
                .with_context(|| format!("reading {}", input.display()))?;
            fs::create_dir_all(&out_dir)?;
            let mut n = 0;
            for line in text
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
            {
                let img = make(line, &icon)?;
                let name: String = line
                    .chars()
                    .filter(char::is_ascii_alphanumeric)
                    .take(42)
                    .collect();
                if name.is_empty() {
                    bail!("cannot derive a file name from {line:?}");
                }
                match format {
                    Format::Svg => fs::write(out_dir.join(format!("{name}.svg")), img.to_svg())?,
                    Format::Png => fs::write(
                        out_dir.join(format!("{name}.png")),
                        img.to_png(usize::from(scale))?,
                    )?,
                }
                n += 1;
            }
            eprintln!("{n} avatars -> {}", out_dir.display());
        }
    }
    Ok(())
}
