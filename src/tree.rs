use super::random::{BinRead, Context, TextRead};

use std::fs::create_dir;
use std::io::{Read, Result};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct NoEntry;

impl std::fmt::Display for NoEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "No entry found")
    }
}

impl std::error::Error for NoEntry {}

pub enum EntryRef<'a> {
    File(&'a FileEntry),
    Dir(&'a DirEntry),
}

pub struct FileEntry {
    pub name: PathBuf,
    pub ino: u64,
    pub parent_ino: u64,
    seed: u64,
    is_binary: bool,
    size: usize,
}

impl FileEntry {
    fn new(
        name: PathBuf,
        ino: u64,
        parent_ino: u64,
        seed: u64,
        is_binary: bool,
        size: usize,
    ) -> Self {
        Self {
            ino,
            parent_ino,
            name,
            seed,
            is_binary,
            size,
        }
    }

    pub fn get_reader(&self) -> Box<dyn Read> {
        if self.is_binary {
            Box::new(BinRead::new(self.seed).take(self.size as u64))
        } else {
            Box::new(TextRead::new(self.seed).take(self.size as u64))
        }
    }

    fn generate(&self, dir: &Path) -> Result<()> {
        let path = dir.join(&self.name);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        //println!("Generate files with {} bytes", self.size);
        std::io::copy(&mut self.get_reader(), &mut file)?;
        Ok(())
    }

    fn size(&self) -> usize {
        self.size
    }

    fn get_entry(&self, ino: u64) -> std::result::Result<EntryRef, NoEntry> {
        if ino == self.ino {
            Ok(EntryRef::File(self))
        } else {
            Err(NoEntry)
        }
    }

    #[cfg(not(windows))]
    pub fn get_attr(&self) -> fuser::FileAttr {
        fuser::FileAttr {
            ino: self.ino,
            size: self.size as u64,
            kind: fuser::FileType::RegularFile,
            blocks: 1,
            atime: std::time::UNIX_EPOCH,
            mtime: std::time::UNIX_EPOCH,
            ctime: std::time::UNIX_EPOCH,
            crtime: std::time::UNIX_EPOCH,
            perm: 0o555,
            nlink: 2,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            blksize: 0,
            flags: 0,
        }
    }
}

pub struct DirEntry {
    pub name: PathBuf,
    pub ino: u64,
    pub parent_ino: u64,
    files: Vec<FileEntry>,
    dirs: Vec<DirEntry>,
}

impl DirEntry {
    pub fn new_root(context: Context) -> Self {
        let (_, s) = Self::new("".into(), 1, 1, context);
        s
    }
    pub(crate) fn new(name: PathBuf, ino: u64, parent_ino: u64, context: Context) -> (u64, Self) {
        let mut current_ino = ino;
        let (nb_files, nb_dir) = context.nb_child();
        let dirs = (0..nb_dir)
            .map(|_| {
                let child_name: PathBuf = context.name().into();
                let (last_ino, d) =
                    DirEntry::new(child_name, current_ino + 1, ino, context.descent());
                current_ino = last_ino;
                d
            })
            .collect();
        let files = (0..nb_files)
            .map(|_| {
                let mut child_name: PathBuf = context.name().into();
                current_ino += 1;
                if context.is_binary() {
                    child_name.set_extension("bin");
                    FileEntry::new(
                        child_name,
                        current_ino,
                        ino,
                        context.get(),
                        true,
                        context.file_len(),
                    )
                } else {
                    child_name.set_extension("text");
                    FileEntry::new(
                        child_name,
                        current_ino,
                        ino,
                        context.get(),
                        false,
                        context.text_len(),
                    )
                }
            })
            .collect();
        (
            current_ino,
            Self {
                name,
                ino,
                parent_ino,
                files,
                dirs,
            },
        )
    }

    pub fn generate(&self, dir: &Path) -> Result<()> {
        let path = dir.join(&self.name);
        create_dir(&path)?;
        for dir in &self.dirs {
            dir.generate(&path)?;
        }
        for file in &self.files {
            file.generate(&path)?;
        }
        Ok(())
    }

    pub fn nb_files(&self) -> u64 {
        let nb_files = self.files.len() as u64;
        nb_files + self.dirs.iter().map(|d| d.nb_files()).sum::<u64>()
    }

    pub fn size(&self) -> usize {
        let file_size = self.files.iter().map(|f| f.size()).sum::<usize>();
        let dir_size = self.dirs.iter().map(|d| d.size()).sum::<usize>();
        file_size + dir_size
    }

    pub fn get_entry(&self, ino: u64) -> std::result::Result<EntryRef, NoEntry> {
        if ino == self.ino {
            Ok(EntryRef::Dir(self))
        } else {
            for file in &self.files {
                if let Ok(r) = file.get_entry(ino) {
                    return Ok(r);
                }
            }
            for dir in &self.dirs {
                if let Ok(r) = dir.get_entry(ino) {
                    return Ok(r);
                }
            }
            Err(NoEntry)
        }
    }

    pub fn get_child(&self, name: &Path) -> std::result::Result<EntryRef, NoEntry> {
        for file in &self.files {
            if file.name == name {
                return Ok(EntryRef::File(file));
            }
        }
        for dir in &self.dirs {
            if dir.name == name {
                return Ok(EntryRef::Dir(dir));
            }
        }
        Err(NoEntry)
    }

    pub fn get_child_idx(&self, mut idx: usize) -> std::result::Result<EntryRef, NoEntry> {
        if idx < self.files.len() {
            Ok(EntryRef::File(&self.files[idx]))
        } else {
            idx -= self.files.len();
            if idx < self.dirs.len() {
                Ok(EntryRef::Dir(&self.dirs[idx]))
            } else {
                Err(NoEntry)
            }
        }
    }

    pub fn get_nb_children(&self) -> usize {
        self.files.len() + self.dirs.len()
    }

    #[cfg(not(windows))]
    pub fn get_attr(&self) -> fuser::FileAttr {
        fuser::FileAttr {
            ino: self.ino,
            size: 0,
            kind: fuser::FileType::Directory,
            blocks: 1,
            atime: std::time::UNIX_EPOCH,
            mtime: std::time::UNIX_EPOCH,
            ctime: std::time::UNIX_EPOCH,
            crtime: std::time::UNIX_EPOCH,
            perm: 0o555,
            nlink: 2,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            blksize: 0,
            flags: 0,
        }
    }
}
