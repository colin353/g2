pub struct BranchFilesystem;

impl BranchFilesystem {
    pub fn new(name: String) -> Self {
        Self {}
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
        println!("readdir branch");
    }
}
