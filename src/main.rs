use rand::distributions::Alphanumeric;
use rand::prelude::*;
use std::cell::RefCell;
use std::fs::create_dir;
use std::io::{Read, Result};
use std::ops::{DerefMut, Range};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use clap::Parser;

struct BinRead<'a> {
    rng: &'a RefCell<SmallRng>,
    len: usize,
}

impl<'a> BinRead<'a> {
    fn new(rng: &'a RefCell<SmallRng>, len: usize) -> Self {
        Self { rng, len }
    }
}

impl Read for BinRead<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let to_read_len = std::cmp::min(buf.len(), self.len);
        self.rng.borrow_mut().fill_bytes(&mut buf[..to_read_len]);
        self.len -= to_read_len;
        Ok(to_read_len)
    }
}

struct TextRead<'a> {
    rng: &'a RefCell<SmallRng>,
    len: usize,
}

impl<'a> TextRead<'a> {
    fn new(rng: &'a RefCell<SmallRng>, len: usize) -> Self {
        Self { rng, len }
    }
}

impl Read for TextRead<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // let's say that means words len is height
        let mut words_to_generate = std::cmp::min(buf.len() >> 3, self.len);
        let string =
            lipsum::lipsum_words_with_rng(self.rng.borrow_mut().deref_mut(), words_to_generate);
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

struct Context {
    pub dir_depth: Range<u64>,
    pub dir_child: Range<u64>,
    pub ratio_dir: f32,
    pub binary_ratio: f32,
    pub file_len: Range<usize>,
    pub rng: Rc<RefCell<SmallRng>>,
}

impl Context {
    fn nb_child(&self) -> (u64, u64) {
        let nb_child = self.rng.borrow_mut().gen_range(self.dir_child.clone());
        let can_contains_dir = self.rng.borrow_mut().gen_range(self.dir_depth.clone());
        let nb_dir = if can_contains_dir > 0 {
            (nb_child as f32 * self.ratio_dir) as u64
        } else {
            0
        };
        println!("{nb_child} -> {nb_dir}");
        (nb_child - nb_dir, nb_dir)
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
            dir_child: self.dir_child.clone(),
            file_len: self.file_len.clone(),
            rng,
            ..*self
        }
    }

    fn binary_read(&self, len: usize) -> BinRead {
        BinRead::new(&self.rng, len)
    }

    fn text_read(&self, len: usize) -> TextRead {
        TextRead::new(&self.rng, len)
    }
}

struct ContextBuilder {
    seed: u64,
    dir_depth: Range<u64>,
    dir_child: Range<u64>,
    ratio_dir: f32,
    binary_ratio: f32,
    file_len: Range<usize>,
}

impl ContextBuilder {
    fn new() -> Self {
        Self {
            seed: 0,
            dir_depth: 4..6,
            dir_child: 5..15,
            ratio_dir: 0.2,
            binary_ratio: 0.2,
            file_len: 10..1_000_000,
        }
    }

    fn dir_depth(&mut self, dir_depth: Range<u64>) -> &mut Self {
        self.dir_depth = dir_depth;
        self
    }

    fn dir_child(&mut self, dir_child: Range<u64>) -> &mut Self {
        self.dir_child = dir_child;
        self
    }

    fn ratio_dir(&mut self, ratio_dir: f32) -> &mut Self {
        self.ratio_dir = ratio_dir;
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
            dir_child: self.dir_child,
            ratio_dir: self.ratio_dir,
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
    dir_child: Option<Range<u64>>,

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

    cli.dir_depth.map(|v| builder.dir_depth(v));
    cli.dir_child.map(|v| builder.dir_child(v));
    cli.ratio_dir.map(|v| builder.ratio_dir(v));
    cli.binary_ratio.map(|v| builder.binary_ratio(v));
    cli.file_len.map(|v| builder.file_len(v));

    let context = builder.create();
    build_dir(&cli.out_dir, context)
}
