use super::Context;

use std::cell::RefCell;
use std::fs::create_dir;
use std::io::{Read, Result};
use std::ops::DerefMut;
use std::path::PathBuf;

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

pub struct DirEntry {
    path: PathBuf,
    files: Vec<FileEntry>,
    dirs: Vec<DirEntry>,
}

impl DirEntry {
    pub(crate) fn new(mut path: PathBuf, context: Context) -> Self {
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

    pub fn generate(&self) -> Result<()> {
        create_dir(&self.path)?;
        for dir in &self.dirs {
            dir.generate()?;
        }
        for file in &self.files {
            file.generate()?;
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
}
