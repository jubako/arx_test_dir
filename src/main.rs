use rand::distributions::Alphanumeric;
use rand::prelude::*;
use std::cell::RefCell;
use std::fs::create_dir;
use std::io::{Read, Result};
use std::ops::{DerefMut, Range};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use clap::Parser;

struct BinRead(SmallRng);

impl BinRead {
    fn new(rng: SmallRng) -> Self {
        Self(rng)
    }
}

impl Read for BinRead {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let to_read_len = std::cmp::min(buf.len(), 1024);
        self.0.fill_bytes(&mut buf[..to_read_len]);
        Ok(to_read_len)
    }
}

struct TextRead(SmallRng);

impl TextRead {
    fn new(rng: SmallRng) -> Self {
        Self(rng)
    }
}

impl Read for TextRead {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // let's say that means words len is height
        let string = lipsum::lipsum_words_with_rng(&mut self.0, 1024);
        let source_len = std::cmp::min(buf.len(), string.len());
        buf[0..source_len].copy_from_slice(string[0..source_len].as_bytes());
        Ok(source_len)
    }
}

fn name(rng: &mut SmallRng) -> String {
    (0..7).map(|_| rng.sample(Alphanumeric) as char).collect()
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
        name(self.rng.borrow_mut().deref_mut())
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

    fn rng(&self) -> SmallRng {
        SmallRng::seed_from_u64(self.rng.borrow_mut().gen())
    }

    fn binary_read(&self) -> BinRead {
        BinRead::new(self.rng())
    }

    fn text_read(&self) -> TextRead {
        TextRead::new(self.rng())
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

struct FileEntry {
    path: PathBuf,
    source: Box<RefCell<dyn Read>>,
    size: usize,
}

impl FileEntry {
    fn new(path: PathBuf, source: Box<RefCell<dyn Read>>, size: usize) -> Self {
        Self { path, source, size }
    }

    fn generate(&self) -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.path)?;
        std::io::copy(
            &mut self.source.borrow_mut().deref_mut().take(self.size as u64),
            &mut file,
        )?;
        Ok(())
    }

    fn size(&self) -> usize {
        self.size
    }
}

struct DirEntry {
    path: PathBuf,
    files: Vec<FileEntry>,
    dirs: Vec<DirEntry>,
}

impl DirEntry {
    fn new(mut path: PathBuf, context: Context) -> Self {
        let (nb_files, nb_dir) = context.nb_child();
        let dirs = (0..nb_dir)
            .map(|_| {
                let child_name: PathBuf = context.name().into();
                path.push(child_name);
                let d = DirEntry::new(path.clone(), context.descent());
                path.pop();
                d
            })
            .collect();
        let files = (0..nb_files)
            .map(|_| {
                let child_name: PathBuf = context.name().into();
                path.push(child_name);
                let f = if context.is_binary() {
                    path.set_extension("bin");
                    FileEntry::new(
                        path.clone(),
                        Box::new(RefCell::new(context.binary_read())),
                        context.file_len(),
                    )
                } else {
                    path.set_extension("text");
                    FileEntry::new(
                        path.clone(),
                        Box::new(RefCell::new(context.text_read())),
                        context.text_len(),
                    )
                };
                path.pop();
                f
            })
            .collect();
        Self { path, files, dirs }
    }

    fn generate(&self) -> Result<()> {
        create_dir(&self.path)?;
        for dir in &self.dirs {
            dir.generate()?;
        }
        for file in &self.files {
            file.generate()?;
        }
        Ok(())
    }

    fn nb_files(&self) -> u64 {
        let nb_files = self.files.len() as u64;
        nb_files + self.dirs.iter().map(|d| d.nb_files()).sum::<u64>()
    }

    fn size(&self) -> usize {
        let file_size = self.files.iter().map(|f| f.size()).sum::<usize>();
        let dir_size = self.dirs.iter().map(|d| d.size()).sum::<usize>();
        file_size + dir_size
    }
}

fn build_dir(path: &Path, context: Context) -> Result<()> {
    let dir = DirEntry::new(path.into(), context);
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
