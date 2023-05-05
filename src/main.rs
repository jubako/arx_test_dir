mod random;
mod tree;

use random::{Context, ContextBuilder};
use std::io::Result;
use std::ops::Range;
use std::path::{Path, PathBuf};

use clap::Parser;

fn build_dir(path: &Path, context: Context) -> Result<()> {
    let dir = tree::DirEntry::new(path.into(), context);
    let nb_files = dir.nb_files();
    let size = dir.size();
    println!("Generate {nb_files} files for a {size} bytes.");
    dir.generate()?;
    Ok(())
}

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
    out_dir: PathBuf,

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
    build_dir(&cli.out_dir, context)
}
