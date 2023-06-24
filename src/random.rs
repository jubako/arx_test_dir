use rand::distributions::Alphanumeric;
use rand::prelude::*;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{Read, Result};
use std::ops::{DerefMut, Range};
use std::rc::Rc;

pub struct BinRead(SmallRng);

impl BinRead {
    pub fn new(seed: u64) -> Self {
        Self(SmallRng::seed_from_u64(seed))
    }
}

impl Read for BinRead {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let to_read_len = std::cmp::min(buf.len(), 1024);
        self.0.fill_bytes(&mut buf[..to_read_len]);
        Ok(to_read_len)
    }
}

pub struct TextRead {
    rng: SmallRng,
    cache: VecDeque<u8>,
}

impl TextRead {
    pub fn new(seed: u64) -> Self {
        //println!("generate reader with seed {}", seed);
        Self {
            rng: SmallRng::seed_from_u64(seed),
            cache: VecDeque::new(),
        }
    }
}

impl Read for TextRead {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // let's say that means words len is height
        if self.cache.is_empty() {
            //  println!("Gen 1024 words");
            for v in lipsum::lipsum_words_with_rng(&mut self.rng, 1024).as_bytes() {
                self.cache.push_back(*v);
            }
        }
        let source_len = std::cmp::min(buf.len(), self.cache.len());
        buf[0..source_len].copy_from_slice(&self.cache.make_contiguous()[0..source_len]);
        self.cache.drain(0..source_len);
        //println!("Have feed {source_len}");
        Ok(source_len)
    }
}

fn name(rng: &mut SmallRng) -> String {
    (0..7).map(|_| rng.sample(Alphanumeric) as char).collect()
}

#[derive(Debug)]
pub struct Context {
    pub dir_depth: Range<u64>,
    pub nb_dir_child: Range<u64>,
    pub nb_file_child: Range<u64>,
    pub binary_ratio: f32,
    pub file_len: Range<usize>,
    pub rng: Rc<RefCell<SmallRng>>,
}

impl Context {
    pub fn nb_child(&self) -> (u64, u64) {
        let nb_files = self.rng.borrow_mut().gen_range(self.nb_file_child.clone());
        let can_contains_dir = self.rng.borrow_mut().gen_range(self.dir_depth.clone());
        let nb_dir = if can_contains_dir > 0 {
            self.rng.borrow_mut().gen_range(self.nb_dir_child.clone())
        } else {
            0
        };
        (nb_files, nb_dir)
    }

    pub fn is_binary(&self) -> bool {
        self.rng.borrow_mut().gen::<f32>() <= self.binary_ratio
    }

    pub fn file_len(&self) -> usize {
        self.rng.borrow_mut().gen_range(self.file_len.clone())
    }

    pub fn text_len(&self) -> usize {
        self.rng.borrow_mut().gen_range(self.file_len.clone()) >> 3
    }

    pub fn name(&self) -> String {
        name(self.rng.borrow_mut().deref_mut())
    }

    pub fn descent(&self) -> Self {
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

    pub fn get<T>(&self) -> T
    where
        rand::distributions::Standard: rand::distributions::Distribution<T>,
    {
        self.rng.borrow_mut().gen()
    }
}

pub struct ContextBuilder {
    seed: u64,
    dir_depth: Range<u64>,
    nb_dir_child: Range<u64>,
    nb_file_child: Range<u64>,
    binary_ratio: f32,
    file_len: Range<usize>,
}

impl ContextBuilder {
    pub fn new() -> Self {
        Self {
            seed: 0,
            dir_depth: 4..6,
            nb_dir_child: 0..5,
            nb_file_child: 0..10,
            binary_ratio: 0.2,
            file_len: 10..1_000_000,
        }
    }

    pub fn seed(&mut self, seed: u64) -> &mut Self {
        self.seed = seed;
        self
    }

    pub fn dir_depth(&mut self, dir_depth: Range<u64>) -> &mut Self {
        self.dir_depth = dir_depth;
        self
    }

    pub fn nb_dir_child(&mut self, nb_dir_child: Range<u64>) -> &mut Self {
        self.nb_dir_child = nb_dir_child;
        self
    }

    pub fn nb_file_child(&mut self, nb_file_child: Range<u64>) -> &mut Self {
        self.nb_file_child = nb_file_child;
        self
    }

    pub fn binary_ratio(&mut self, binary_ratio: f32) -> &mut Self {
        self.binary_ratio = binary_ratio;
        self
    }

    pub fn file_len(&mut self, file_len: Range<usize>) -> &mut Self {
        self.file_len = file_len;
        self
    }

    pub fn create(self) -> Context {
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

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}
