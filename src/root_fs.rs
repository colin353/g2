use fuse::{FileAttr, FileType};
use libc::ENOENT;
use std::ffi::OsStr;

use std::collections::HashMap;

use crate::fs::attrs;

pub struct RootFilesystem {
    branch_ino_map: HashMap<String, u64>,
    pub ino_branch_map: HashMap<u64, String>,
    last_ino: u64,
}

impl RootFilesystem {
    pub fn new() -> Self {
        Self {
            branch_ino_map: HashMap::new(),
            ino_branch_map: HashMap::new(),
            last_ino: 1,
        }
    }

    fn reserve_ino(&mut self, branch: String) -> u64 {
        let ino = self.next_ino();
        self.branch_ino_map.insert(branch.clone(), ino);
        self.ino_branch_map.insert(ino, branch);
        ino
    }

    fn next_ino(&mut self) -> u64 {
        self.last_ino += 1;

        // TODO: come up with a better idea for this?
        if self.last_ino > crate::fs::BRANCH_INO_LIMIT {
            panic!("exhausted branch inos!");
        }
        self.last_ino
    }
}

impl fuse::Filesystem for RootFilesystem {
    fn lookup(&mut self, _req: &fuse::Request, parent: u64, name: &OsStr, reply: fuse::ReplyEntry) {
        let target = if let Some(s) = name.to_str() {
            s
        } else {
            return reply.error(ENOENT);
        };

        // Check the branch ino map
        if let Some(ino) = self.branch_ino_map.get(target) {
            return reply.entry(
                &time::Timespec::new(120, 0),
                &attrs(*ino, FileType::Directory),
                0,
            );
        }

        // Check if the item exists in ~/.g2/branches
        let root = crate::conf::root_dir();
        if std::path::Path::new(&format!("{}/branches/{}", root, target)).exists() {
            let ino = self.reserve_ino(target.to_string());
            return reply.entry(
                &time::Timespec::new(120, 0),
                &attrs(ino, FileType::Directory),
                0,
            );
        }

        reply.error(ENOENT);
    }

    fn getattr(&mut self, req: &fuse::Request, ino: u64, reply: fuse::ReplyAttr) {
        if ino == 1 {
            return reply.attr(
                &time::Timespec::new(120, 0),
                &attrs(ino, FileType::Directory),
            );
        }

        if let Some(_) = self.ino_branch_map.get(&ino) {
            return reply.attr(
                &time::Timespec::new(120, 0),
                &attrs(ino, FileType::Directory),
            );
        }

        reply.error(ENOENT);
    }

    fn readdir(
        &mut self,
        req: &fuse::Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: fuse::ReplyDirectory,
    ) {
        let config = crate::conf::get_config();
        let mut entries = vec![
            (1, FileType::Directory, "."),
            (1, FileType::Directory, ".."),
        ];
        for branch in &config.branches {
            if let Some(ino) = self.branch_ino_map.get(&branch.name) {
                entries.push((*ino, FileType::Directory, &branch.name));
            } else {
                let ino = self.reserve_ino(branch.name.clone());
                entries.push((ino, FileType::Directory, &branch.name));
            }
        }

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            reply.add(entry.0, (i + 1) as i64, entry.1, entry.2);
        }

        reply.ok();
    }
}
