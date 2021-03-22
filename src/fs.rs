use fuse::{FileAttr, FileType};
use libc::ENOENT;
use std::collections::HashMap;
use std::ffi::OsStr;

use crate::branch_fs::BranchFilesystem;
use crate::root_fs::RootFilesystem;

pub const BRANCH_INO_LIMIT: u64 = 4096;

pub fn attrs(ino: u64, filetype: FileType) -> FileAttr {
    FileAttr {
        ino,
        size: 13,
        blocks: 1,
        atime: time::Timespec::new(0, 0),
        mtime: time::Timespec::new(0, 0),
        ctime: time::Timespec::new(0, 0),
        crtime: time::Timespec::new(0, 0),
        kind: filetype,
        perm: 0o644,
        nlink: 1,
        uid: 501,
        gid: 20,
        rdev: 0,
        flags: 0,
    }
}

pub fn serve() {
    let root_dir = crate::conf::root_dir();
    let mount_dir = format!("{}/fs", root_dir);
    std::fs::create_dir_all(&mount_dir).unwrap();

    let options = &["-o", "fsname=g2", "async_read=true", "negative_timeout=5"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&std::ffi::OsStr>>();

    let filesystem = G2Filesystem::new();

    fuse::mount(filesystem, &mount_dir, &options).unwrap();
}

enum Node {
    Root,
    BranchRoot(usize, String),
}

struct G2Filesystem {
    nodes: HashMap<u64, Node>,
    root_fs: RootFilesystem,
    branch_fs: HashMap<String, BranchFilesystem>,
}

impl G2Filesystem {
    pub fn new() -> Self {
        G2Filesystem {
            nodes: HashMap::new(),
            root_fs: RootFilesystem::new(),
            branch_fs: HashMap::new(),
        }
    }

    pub fn subfilesystem(&mut self, ino: u64) -> Option<&mut dyn fuse::Filesystem> {
        let node = if ino == 1 {
            &Node::Root
        } else if ino <= BRANCH_INO_LIMIT {
            let branch = match self.root_fs.ino_branch_map.get(&ino) {
                Some(b) => b,
                None => return None,
            };

            if !self.branch_fs.contains_key(branch) {
                self.branch_fs.insert(
                    branch.to_string(),
                    BranchFilesystem::new(branch.to_string()),
                );
            }

            if let Some(fs) = self.branch_fs.get_mut(branch) {
                return Some(fs);
            }
            return None;
        } else {
            match self.nodes.get(&ino) {
                Some(x) => x,
                None => return None,
            }
        };

        match node {
            _ => Some(&mut self.root_fs),
            /*Node::Root => &mut self.root_fs,
            Node::Branch(bno, path) => &mut BranchFilesystem {},*/
        }
    }
}

impl fuse::Filesystem for G2Filesystem {
    fn lookup(&mut self, req: &fuse::Request, parent: u64, name: &OsStr, reply: fuse::ReplyEntry) {
        let fs = match self.subfilesystem(parent) {
            Some(f) => f,
            None => return reply.error(ENOENT),
        };
        fs.lookup(req, parent, name, reply);
    }

    fn getattr(&mut self, req: &fuse::Request, ino: u64, reply: fuse::ReplyAttr) {
        let mut fs = match self.subfilesystem(ino) {
            Some(f) => f,
            None => return reply.error(ENOENT),
        };
        fs.getattr(req, ino, reply);
    }

    fn read(
        &mut self,
        _req: &fuse::Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        _size: u32,
        reply: fuse::ReplyData,
    ) {
        println!("read: {}", ino);
        if ino == 2 {
            reply.data("asdf\n".as_bytes());
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(
        &mut self,
        req: &fuse::Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: fuse::ReplyDirectory,
    ) {
        let fs = match self.subfilesystem(ino) {
            Some(f) => f,
            None => return reply.error(ENOENT),
        };
        fs.readdir(req, ino, _fh, offset, reply);
    }
}
