mod mount;
mod random;
mod tree;

use random::ContextBuilder;
use std::io::Result;
use std::ops::Range;
use std::path::PathBuf;

use clap::Parser;

fn parse_range<T>(s: &str) -> std::result::Result<Range<T>, String>
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Debug,
{
    let (start, end) = s.split_once("..").ok_or(format!("'{s}' is not a range"))?;
    let start = start
        .parse::<T>()
        .map_err(|e| format!("'{start}' is not a valid value ({e:?})"))?;
    let end = end
        .parse::<T>()
        .map_err(|e| format!("'{end}' is not a valid value ({e:?})"))?;
    Ok(start..end)
}

fn parse_range_64(s: &str) -> std::result::Result<Range<u64>, String> {
    parse_range(s)
}

fn parse_range_usize(s: &str) -> std::result::Result<Range<usize>, String> {
    parse_range(s)
}

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    extract_dir: Option<PathBuf>,

    #[arg(long)]
    mount_dir: Option<PathBuf>,

    #[arg(long, short)]
    seed: Option<u64>,

    #[arg(long, value_parser = parse_range_64)]
    dir_depth: Option<Range<u64>>,

    #[arg(long, value_parser = parse_range_64)]
    nb_dir_child: Option<Range<u64>>,

    #[arg(long, value_parser = parse_range_64)]
    nb_file_child: Option<Range<u64>>,

    #[arg(long)]
    ratio_dir: Option<f32>,

    #[arg(long)]
    binary_ratio: Option<f32>,

    #[arg(long, value_parser = parse_range_usize)]
    file_len: Option<Range<usize>>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut builder = ContextBuilder::new();

    cli.seed.map(|v| builder.seed(v));
    cli.dir_depth.map(|v| builder.dir_depth(v));
    cli.nb_dir_child.map(|v| builder.nb_dir_child(v));
    cli.nb_file_child.map(|v| builder.nb_file_child(v));
    cli.binary_ratio.map(|v| builder.binary_ratio(v));
    cli.file_len.map(|v| builder.file_len(v));

    let context = builder.create();

    println!("Generating with {context:?}");

    let (_, dir) = tree::DirEntry::new("".into(), 1, 1, context);
    let nb_files = dir.nb_files();
    let size = dir.size();
    println!("Generate {nb_files} files for a {size} bytes.");

    if let Some(path) = cli.extract_dir {
        dir.generate(&path)?;
    }

    if let Some(path) = cli.mount_dir {
        let options = vec![
            fuser::MountOption::RO,
            fuser::MountOption::FSName("test_arx".into()),
        ];
        fuser::mount2(mount::TreeFs::new(dir), path, &options)?;
    }

    Ok(())
}
