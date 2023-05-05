use rand::distributions::Alphanumeric;
use rand::prelude::*;
use std::cell::RefCell;
use std::fs::create_dir;
use std::io::{Read, Result};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::ops::DerefMut;

struct BinRead<'a> {
    rng: &'a RefCell<SmallRng>,
    len: usize
}

impl<'a> BinRead<'a> {
    fn new(rng: &'a RefCell<SmallRng>, len: usize) -> Self {
        Self {
            rng, len
        }
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
    len: usize
}

impl<'a> TextRead<'a> {
    fn new(rng: &'a RefCell<SmallRng>, len: usize) -> Self {
        Self {
            rng, len
        }
    }
}

impl Read for TextRead<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // let's say that means words len is height
        let mut words_to_generate = std::cmp::min(buf.len()>>3, self.len);
        let string = lipsum::lipsum_words_with_rng(self.rng.borrow_mut().deref_mut(), words_to_generate);
        let mut source_len = string.len();
        loop {
            match string[0..source_len].rfind(' ') {
                None => {break}
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
    pub dir_depth: (u64, u64),
    pub dir_child: (u64, u64),
    pub ratio_dir: f32,
    pub binary_ratio: f32,
    pub file_len: (usize, usize),
    pub rng: Rc<RefCell<SmallRng>>,
}

impl Context {
    fn new(
        dir_depth: (u64, u64),
        dir_child: (u64, u64),
        ratio_dir: f32,
        binary_ratio: f32,
        file_len: (usize, usize),
    ) -> Self {
        let rng = Rc::new(RefCell::new(SmallRng::seed_from_u64(5)));
        Self {
            dir_depth,
            dir_child,
            ratio_dir,
            binary_ratio,
            file_len,
            rng,
        }
    }

    fn nb_child(&self) -> (u64, u64) {
        let nb_child = self
            .rng
            .borrow_mut()
            .gen_range(self.dir_child.0..=self.dir_child.1);
        let can_contains_dir = self.rng.borrow_mut().gen_range(self.dir_depth.0..=self.dir_depth.1);
        let nb_dir = if can_contains_dir > 0 {
            (nb_child as f32 * self.ratio_dir) as u64
        } else {
            0
        };
        println!("{nb_child} -> {nb_dir}");
        (nb_child - nb_dir, nb_dir)
    }

    fn is_binary(&self) -> bool {
        self.rng
                    .borrow_mut()
                    .gen::<f32>() <= self.binary_ratio
    }

    fn file_len(&self) -> usize {
        self.rng
            .borrow_mut()
            .gen_range(self.file_len.0..=self.file_len.1)
    }

    fn text_len(&self) -> usize {
        self.rng
        .borrow_mut()
        .gen_range(self.file_len.0..=self.file_len.1) >> 3
    }

    fn name(&self) -> String {
        (0..7)
            .map(|_| self.rng.borrow_mut().sample(Alphanumeric) as char)
            .collect()
    }

    fn descent(&self) -> Self {
        let dir_depth = (
            self.dir_depth.0.saturating_sub(1),
            self.dir_depth.1.saturating_sub(1),
        );
        let rng = Rc::clone(&self.rng);
        Self {
            dir_depth,
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
    std::io::copy(&mut random, &mut file)?;
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

fn main() {
    let context = Context::new((4, 6), (5, 15), 0.2, 0.2, (10, 1000_0000));
    let path = PathBuf::from("GEN_TEST");
    build_dir(&path, context).unwrap();
}
