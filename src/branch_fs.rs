use fuse::{FileAttr, FileType};
use libc::ENOENT;
use std::ffi::OsStr;
use std::io::{Read, Seek, SeekFrom};

use std::cell::RefCell;
use std::collections::HashMap;

use crate::fs::{attrs, Node};

pub struct BranchFilesystem {
    nodes: RefCell<HashMap<u64, Node>>,
    branch_id: usize,
    root_ino: u64,
    temp_dir: String,
    ino_to_path: HashMap<u64, String>,
    path_to_ino: HashMap<String, u64>,
}

fn attrs_from_metadata(ino: u64, meta: &std::fs::Metadata) -> FileAttr {
    let file_type = match meta.file_type().is_file() {
        true => FileType::RegularFile,
        false => FileType::Directory,
    };
    attrs(ino, file_type)
}

impl BranchFilesystem {
    pub fn new(
        root_ino: u64,
        name: String,
        nodes: RefCell<HashMap<u64, Node>>,
        branch_id: usize,
    ) -> Self {
        let temp_dir = format!("{}/branches/{}", crate::conf::root_dir(), name);

        let mut s = Self {
            temp_dir,
            ino_to_path: HashMap::new(),
            path_to_ino: HashMap::new(),
            root_ino,
            nodes,
            branch_id,
        };
        s.path_to_ino.insert(String::new(), root_ino);
        s.ino_to_path.insert(root_ino, String::new());

        s
    }

    pub fn reserve_ino(&mut self, path: &str) -> u64 {
        if let Some(x) = self.path_to_ino.get(path) {
            return *x;
        }

        let ino = crate::fs::generate_ino();

        RefCell::get_mut(&mut self.nodes).insert(ino, Node::BranchContent(self.branch_id));

        self.path_to_ino.insert(path.to_owned(), ino);
        self.ino_to_path.insert(ino, path.to_owned());
        ino
    }
}

impl fuse::Filesystem for BranchFilesystem {
    fn readdir(
        &mut self,
        req: &fuse::Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: fuse::ReplyDirectory,
    ) {
        let path = if ino == self.root_ino {
            String::new()
        } else {
            match self.ino_to_path.get(&ino) {
                Some(p) => p.to_string(),
                None => return reply.error(ENOENT),
            }
        };

        let mut entries = vec![
            (ino, FileType::Directory, String::from(".")),
            (1, FileType::Directory, String::from("..")),
        ];

        let maybe_temp_iter = std::fs::read_dir(format!("{}/{}", self.temp_dir, path));
        if let Ok(temp_iter) = maybe_temp_iter {
            for item in temp_iter {
                let item = item.unwrap();
                let filename = item.file_name().into_string().unwrap();
                let item_path = format!("{}/{}", path, filename);
                let ino = self.reserve_ino(&item_path);
                let file_type = match item.file_type().unwrap().is_file() {
                    true => FileType::RegularFile,
                    false => FileType::Directory,
                };
                entries.push((ino, file_type, filename));
            }
        }

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            reply.add(entry.0, (i + 1) as i64, entry.1, entry.2);
        }

        reply.ok()
    }

    fn read(
        &mut self,
        _req: &fuse::Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        reply: fuse::ReplyData,
    ) {
        let path = match self.ino_to_path.get(&ino) {
            Some(p) => p,
            None => return reply.error(ENOENT),
        };

        if let Ok(mut f) = std::fs::File::open(format!("{}/{}", self.temp_dir, path)) {
            let seek_position = if offset >= 0 {
                SeekFrom::Start(offset as u64)
            } else {
                SeekFrom::End(offset)
            };
            if f.seek(seek_position).is_err() {
                return reply.error(ENOENT);
            }

            let mut buf = Vec::with_capacity(size as usize);
            if f.read(&mut buf).is_err() {
                return reply.error(ENOENT);
            }
            return reply.data(&buf);
        }

        reply.error(ENOENT);
    }

    fn lookup(&mut self, _req: &fuse::Request, parent: u64, name: &OsStr, reply: fuse::ReplyEntry) {
        let path = match self.ino_to_path.get(&parent) {
            Some(p) => format!("{}/{}", p, name.to_str().unwrap()),
            None => return reply.error(ENOENT),
        };
        let ino = self.reserve_ino(&path);
        if let Ok(temp_meta) = std::fs::metadata(format!("{}/{}", self.temp_dir, path)) {
            let file_type = match temp_meta.file_type().is_file() {
                true => FileType::RegularFile,
                false => FileType::Directory,
            };
            return reply.entry(
                &time::Timespec::new(120, 0),
                &crate::fs::attrs(ino, file_type),
                0,
            );
        }

        reply.error(ENOENT);
    }

    fn getattr(&mut self, req: &fuse::Request, ino: u64, reply: fuse::ReplyAttr) {
        if let Some(path) = self.ino_to_path.get(&ino) {
            if let Ok(metadata) = std::fs::metadata(format!("{}/{}", self.temp_dir, path)) {
                return reply.attr(
                    &time::Timespec::new(120, 0),
                    &attrs_from_metadata(ino, &metadata),
                );
            };
        }

        reply.error(ENOENT);
    }
}
