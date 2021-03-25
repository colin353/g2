use std::collections::HashMap;

pub struct BranchFilesystem {
    root_ino: u64,
    temp_dir: String,
    inos: HashMap<u64, String>,
}

impl BranchFilesystem {
    pub fn new(root_ino: u64, name: String) -> Self {
        let temp_dir = format!("{}/branches/{}", crate::conf::root_dir(), name);

        Self {
            temp_dir,
            inos: HashMap::new(),
            root_ino,
        }
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
        if ino == self.root_ino {
            std::fs::read_dir(&self.temp_dir).unwrap();
        }
        println!("readdir branch");
    }
}
