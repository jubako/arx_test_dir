use rand::distributions::Alphanumeric;
use rand::prelude::*;
use std::cell::RefCell;
use std::fs::create_dir;
use std::io::{Read, Result};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use clap::Parser;

struct BinRead {
    rng: SmallRng,
    len: usize,
}

impl BinRead {
    fn new(seed: u64, len: usize) -> Self {
        Self {
            rng: SmallRng::seed_from_u64(seed),
            len,
        }
    }
}

impl Read for BinRead {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let to_read_len = std::cmp::min(buf.len(), self.len);
        self.rng.fill_bytes(&mut buf[..to_read_len]);
        self.len -= to_read_len;
        Ok(to_read_len)
    }
}

struct TextRead {
    rng: SmallRng,
    len: usize,
}

impl TextRead {
    fn new(seed: u64, len: usize) -> Self {
        Self {
            rng: SmallRng::seed_from_u64(seed),
            len,
        }
    }
}

impl Read for TextRead {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // let's say that means words len is height
        let mut words_to_generate = std::cmp::min(buf.len() >> 3, self.len);
        let string = lipsum::lipsum_words_with_rng(&mut self.rng, words_to_generate);
        let mut source_len = string.len();
        loop {
            match string[0..source_len].rfind(' ') {
                None => break,
                Some(i) => {
                    source_len = i;
                    words_to_generate -= 1;
                    if i < buf.len() {
                        break;
                    }
                }
            }
        }
        buf[0..source_len].copy_from_slice(string[0..source_len].as_bytes());
        self.len -= words_to_generate;
        Ok(source_len)
    }
}

#[derive(Debug)]
struct Context {
    pub dir_depth: Range<u64>,
    pub nb_dir_child: Range<u64>,
    pub nb_file_child: Range<u64>,
    pub binary_ratio: f32,
    pub file_len: Range<usize>,
    pub rng: Rc<RefCell<SmallRng>>,
}

impl Context {
    fn nb_child(&self) -> (u64, u64) {
        let nb_files = self.rng.borrow_mut().gen_range(self.nb_file_child.clone());
        let can_contains_dir = self.rng.borrow_mut().gen_range(self.dir_depth.clone());
        let nb_dir = if can_contains_dir > 0 {
            self.rng.borrow_mut().gen_range(self.nb_dir_child.clone())
        } else {
            0
        };
        (nb_files, nb_dir)
    }

    fn is_binary(&self) -> bool {
        self.rng.borrow_mut().gen::<f32>() <= self.binary_ratio
    }

    fn file_len(&self) -> usize {
        self.rng.borrow_mut().gen_range(self.file_len.clone())
    }

    fn text_len(&self) -> usize {
        self.rng.borrow_mut().gen_range(self.file_len.clone()) >> 3
    }

    fn name(&self) -> String {
        (0..7)
            .map(|_| self.rng.borrow_mut().sample(Alphanumeric) as char)
            .collect()
    }

    fn descent(&self) -> Self {
        let dir_depth =
            self.dir_depth.start.saturating_sub(1)..self.dir_depth.end.saturating_sub(1);
        let rng = Rc::clone(&self.rng);
        Self {
            dir_depth,
            nb_dir_child: self.nb_dir_child.clone(),
            nb_file_child: self.nb_file_child.clone(),
            file_len: self.file_len.clone(),
            rng,
            ..*self
        }
    }

    fn binary_read(&self, len: usize) -> BinRead {
        BinRead::new(self.rng.borrow_mut().gen(), len)
    }

    fn text_read(&self, len: usize) -> TextRead {
        TextRead::new(self.rng.borrow_mut().gen(), len)
    }
}

struct ContextBuilder {
    seed: u64,
    dir_depth: Range<u64>,
    nb_dir_child: Range<u64>,
    nb_file_child: Range<u64>,
    binary_ratio: f32,
    file_len: Range<usize>,
}

impl ContextBuilder {
    fn new() -> Self {
        Self {
            seed: 0,
            dir_depth: 4..6,
            nb_dir_child: 0..5,
            nb_file_child: 0..10,
            binary_ratio: 0.2,
            file_len: 10..1_000_000,
        }
    }

    fn seed(&mut self, seed: u64) -> &mut Self {
        self.seed = seed;
        self
    }

    fn dir_depth(&mut self, dir_depth: Range<u64>) -> &mut Self {
        self.dir_depth = dir_depth;
        self
    }

    fn nb_dir_child(&mut self, nb_dir_child: Range<u64>) -> &mut Self {
        self.nb_dir_child = nb_dir_child;
        self
    }

    fn nb_file_child(&mut self, nb_file_child: Range<u64>) -> &mut Self {
        self.nb_file_child = nb_file_child;
        self
    }

    fn binary_ratio(&mut self, binary_ratio: f32) -> &mut Self {
        self.binary_ratio = binary_ratio;
        self
    }

    fn file_len(&mut self, file_len: Range<usize>) -> &mut Self {
        self.file_len = file_len;
        self
    }

    fn create(self) -> Context {
        Context {
            dir_depth: self.dir_depth,
            nb_dir_child: self.nb_dir_child,
            nb_file_child: self.nb_file_child,
            binary_ratio: self.binary_ratio,
            file_len: self.file_len,
            rng: Rc::new(RefCell::new(SmallRng::seed_from_u64(self.seed))),
        }
    }
}
fn build_random_file(path: &Path, context: &Context) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    let len = context.file_len();
    let mut random = context.binary_read(len);
    std::io::copy(&mut random, &mut file)?;
    Ok(())
}

fn build_text_file(path: &Path, context: &Context) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    let len = context.text_len();
    let mut random = context.text_read(len);
    let size = std::io::copy(&mut random, &mut file)?;
    assert!(size <= context.file_len.end as u64);
    Ok(())
}

fn build_dir(path: &Path, context: Context) -> Result<()> {
    create_dir(path)?;
    let mut path: PathBuf = path.into();
    let (nb_files, nb_dir) = context.nb_child();
    for _i in 0..nb_dir {
        let child_name: PathBuf = context.name().into();
        path.push(child_name);
        build_dir(&path, context.descent())?;
        path.pop();
    }
    for _i in 0..nb_files {
        let child_name: PathBuf = context.name().into();
        path.push(child_name);
        if context.is_binary() {
            path.set_extension("bin");
            build_random_file(&path, &context)?;
        } else {
            path.set_extension("text");
            build_text_file(&path, &context)?;
        }
        path.pop();
    }
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
