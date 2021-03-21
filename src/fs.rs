use fuse::{FileAttr, FileType};
use libc::ENOENT;
use std::collections::HashMap;
use std::ffi::OsStr;

fn attrs(filetype: FileType) -> FileAttr {
    FileAttr {
        ino: 2,
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

    fuse::mount(filesystem, &mount_dir, &options);
}

enum Node {
    Root,
    Branch(usize, String),
}

struct G2Filesystem {
    nodes: HashMap<u64, Node>,
}

struct RootFilesystem;
struct BranchFilesystem;

impl G2Filesystem {
    pub fn new() -> Self {
        G2Filesystem {
            nodes: HashMap::new(),
        }
    }

    pub fn subfilesystem(&self, node: &Node) -> Box<dyn fuse::Filesystem> {
        match node {
            Node::Root => Box::new(RootFilesystem {}),
            Node::Branch(bno, path) => Box::new(BranchFilesystem {}),
        }
    }

    pub fn lookup_ino(&self, ino: u64) -> Option<&Node> {
        if ino == 1 {
            return Some(&Node::Root);
        }
        self.nodes.get(&ino)
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
    }
}

impl fuse::Filesystem for RootFilesystem {
    fn lookup(&mut self, _req: &fuse::Request, parent: u64, name: &OsStr, reply: fuse::ReplyEntry) {
        if name.to_str() == Some("hello.txt") {
            reply.entry(&time::Timespec::new(1, 0), &attrs(FileType::RegularFile), 0);
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
        let entries = vec![
            (1, FileType::Directory, "."),
            (1, FileType::Directory, ".."),
            (2, FileType::RegularFile, "hello.txt"),
        ];
        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            reply.add(entry.0, (i + 1) as i64, entry.1, entry.2);
        }
        reply.ok();
    }
}

impl fuse::Filesystem for G2Filesystem {
    fn lookup(&mut self, req: &fuse::Request, parent: u64, name: &OsStr, reply: fuse::ReplyEntry) {
        println!("lookup: {}", parent);
        let node = match self.lookup_ino(parent) {
            Some(n) => n,
            None => return reply.error(ENOENT),
        };
        let mut fs = self.subfilesystem(node);
        fs.lookup(req, parent, name, reply);
    }

    fn getattr(&mut self, _req: &fuse::Request, ino: u64, reply: fuse::ReplyAttr) {
        println!("getattr: {}", ino);
        match ino {
            1 => reply.attr(&time::Timespec::new(1, 0), &attrs(FileType::Directory)),
            2 => reply.attr(&time::Timespec::new(1, 0), &attrs(FileType::RegularFile)),
            _ => reply.error(ENOENT),
        }
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
        println!("readdir: {}", ino);
        let node = match self.lookup_ino(ino) {
            Some(n) => n,
            None => return reply.error(ENOENT),
        };
        let mut fs = self.subfilesystem(node);
        fs.readdir(req, ino, _fh, offset, reply);
    }
}
