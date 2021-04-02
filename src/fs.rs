use fuse::{FileAttr, FileType};
use libc::ENOENT;
use rand::Rng;
use std::collections::HashMap;
use std::ffi::OsStr;

use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock};

use crate::branch_fs::BranchFilesystem;
use crate::root_fs::RootFilesystem;

pub const BRANCH_INO_LIMIT: u64 = 4096;
const NEXT_INO: AtomicU64 = AtomicU64::new(BRANCH_INO_LIMIT + 1);

pub fn generate_ino() -> u64 {
    NEXT_INO.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

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

#[derive(Debug, Clone)]
pub enum Node {
    Root,
    BranchContent(usize),
}

struct G2Filesystem {
    nodes: Arc<RwLock<HashMap<u64, Node>>>,
    root_fs: RootFilesystem,
    branches: Vec<BranchFilesystem>,
    name_to_branch: HashMap<String, usize>,
}

impl G2Filesystem {
    pub fn new() -> Self {
        G2Filesystem {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            root_fs: RootFilesystem::new(),
            branches: Vec::new(),
            name_to_branch: HashMap::new(),
        }
    }

    pub fn subfilesystem(&mut self, ino: u64) -> Option<&mut dyn fuse::Filesystem> {
        let nodes = self.nodes.read().unwrap();
        let node = if ino == 1 {
            &Node::Root
        } else if ino <= BRANCH_INO_LIMIT {
            println!("under branch limit {}", ino);
            let branch = match self.root_fs.ino_branch_map.get(&ino) {
                Some(b) => b,
                None => return None,
            };

            if !self.name_to_branch.contains_key(branch) {
                let bid = self.branches.len();
                self.branches.push(BranchFilesystem::new(
                    ino,
                    branch.to_string(),
                    self.nodes.clone(),
                    bid,
                ));
                self.name_to_branch.insert(branch.to_string(), bid);
            }

            if let Some(bid) = self.name_to_branch.get(branch) {
                return Some(&mut self.branches[*bid]);
            }
            return None;
        } else {
            println!("over branch limit: {}", ino);
            match nodes.get(&ino) {
                Some(x) => x,
                None => return None,
            }
        };
        println!("node: {:?}", node);

        match node {
            Node::Root => Some(&mut self.root_fs),
            Node::BranchContent(bid) => Some(&mut self.branches[*bid]),
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
        req: &fuse::Request,
        ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        reply: fuse::ReplyData,
    ) {
        println!("read root, ino {}", ino);
        let fs = match self.subfilesystem(ino) {
            Some(f) => f,
            None => return reply.error(ENOENT),
        };
        fs.read(req, ino, fh, offset, size, reply);
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
