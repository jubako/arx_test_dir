use super::tree::{DirEntry, EntryRef};
use std::ffi::OsStr;
use std::io::Read;
use std::path::Path;

const TTL: std::time::Duration = std::time::Duration::from_secs(1000); // Nothing change on oar side, TTL is long

pub struct TreeFs {
    root: DirEntry,
}

impl TreeFs {
    pub fn new(root: DirEntry) -> Self {
        Self { root }
    }
}

impl fuser::Filesystem for TreeFs {
    fn lookup(
        &mut self,
        _req: &fuser::Request,
        parent: u64,
        name: &OsStr,
        reply: fuser::ReplyEntry,
    ) {
        //        println!("Lookup for {name:?} in {parent}");
        match self.root.get_entry(parent) {
            Ok(entry) => match entry {
                EntryRef::File(_) => reply.error(libc::ENOENT),
                EntryRef::Dir(d) => {
                    //                  println!("  Parent is {:?}", d.name);
                    match d.get_child(Path::new(name)) {
                        Ok(child) => match child {
                            EntryRef::File(f) => {
                                //                            println!("    Found file {:?}", f.name);
                                reply.entry(&TTL, &f.get_attr(), 0)
                            }
                            EntryRef::Dir(d) => {
                                //                          println!("    Found dir {:?}", d.name);
                                reply.entry(&TTL, &d.get_attr(), 0)
                            }
                        },
                        Err(_) => reply.error(libc::ENOENT),
                    }
                }
            },
            Err(_) => reply.error(libc::ENOENT),
        }
    }

    fn getattr(&mut self, _req: &fuser::Request, ino: u64, reply: fuser::ReplyAttr) {
        match self.root.get_entry(ino).unwrap() {
            EntryRef::File(f) => reply.attr(&TTL, &f.get_attr()),
            EntryRef::Dir(d) => reply.attr(&TTL, &d.get_attr()),
        }
    }
    /*
    fn readlink(&mut self, _req: &fuser::Request, ino:u64, reply: fuser::ReplyData) {

    }*/

    fn open(&mut self, _req: &fuser::Request, ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        match self.root.get_entry(ino).unwrap() {
            EntryRef::File(_) => reply.opened(0, 0),
            EntryRef::Dir(_) => reply.error(libc::EISDIR),
        }
    }

    fn opendir(&mut self, _req: &fuser::Request, ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        match self.root.get_entry(ino).unwrap() {
            EntryRef::Dir(_) => reply.opened(0, 0),
            EntryRef::File(_) => reply.error(libc::ENOTDIR),
        }
    }

    fn read(
        &mut self,
        _req: &fuser::Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyData,
    ) {
        match self.root.get_entry(ino).unwrap() {
            EntryRef::File(f) => {
                let reader = f.get_reader();
                //println!("Skip {offset} bytes");
                let mut skip_reader = reader.take(offset as u64);
                std::io::copy(&mut skip_reader, &mut std::io::sink()).unwrap();

                //println!("Read {size} bytes");
                let mut reader = skip_reader.into_inner().take(size.into());
                let mut data = Vec::new();
                reader.read_to_end(&mut data).unwrap();
                //println!("Data size is {}", data.len());
                reply.data(&data)
            }
            EntryRef::Dir(_) => reply.error(libc::EISDIR),
        }
    }

    fn readdir(
        &mut self,
        _req: &fuser::Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: fuser::ReplyDirectory,
    ) {
        match self.root.get_entry(ino).unwrap() {
            EntryRef::File(_) => reply.error(libc::ENOTDIR),
            EntryRef::Dir(d) => {
                let nb_entry = d.get_nb_children() as i64 + 2; // we include "." and ".."
                let offset = if offset == 0 { 0 } else { offset + 1 };

                /*println!(
                    "Listing child in {:?} (ino {}) (offset:{offset})",
                    d.name, d.ino
                );*/
                for i in offset..nb_entry {
                    if i == 0 {
                        //                        println!(" - '.' (dir) ino:{}, id: {i}", d.ino);
                        if reply.add(d.ino, i, fuser::FileType::Directory, ".") {
                            break;
                        }
                    } else if i == 1 {
                        //                      println!(" - '..' (dir) ino:{}, id: {i}", d.parent_ino);
                        if reply.add(d.parent_ino, i, fuser::FileType::Directory, "..") {
                            break;
                        }
                    } else {
                        match d.get_child_idx((i - 2) as usize).unwrap() {
                            EntryRef::File(f) => {
                                //                            println!(" - '{:?}' (file) ino:{}, id: {i}", f.name, f.ino);
                                if reply.add(f.ino, i, fuser::FileType::RegularFile, f.name.clone())
                                {
                                    break;
                                }
                            }
                            EntryRef::Dir(d) => {
                                //                          println!(" - '{:?}' (dir) ino:{}, id: {i}", d.name, d.ino);
                                if reply.add(d.ino, i, fuser::FileType::Directory, d.name.clone()) {
                                    break;
                                }
                            }
                        }
                    }
                }
                reply.ok()
            }
        }
    }
}
