use antigen_core::{Construct, MessageContext, MessageResult, Usage};
use std::path::PathBuf;

pub enum FilePath {}
pub enum FileBytes {}
pub enum FileString {}

pub type FilePathComponent = Usage<FilePath, PathBuf>;

pub type FileBytesComponent = Usage<FileBytes, Vec<u8>>;
pub type FileStringComponent = Usage<FileString, String>;

#[derive(hecs::Bundle)]
pub struct FileBytesBundle {
    path: FilePathComponent,
    bytes: FileBytesComponent,
}

impl FileBytesBundle {
    pub fn new<P: Into<PathBuf>, B: Into<Vec<u8>>>(path: P, bytes: B) -> Self {
        let path = FilePathComponent::construct(path.into());
        let bytes = FileBytesComponent::construct(bytes.into());

        FileBytesBundle { path, bytes }
    }
}

#[derive(hecs::Bundle)]
pub struct FileStringBundle {
    path: FilePathComponent,
    string: FileStringComponent,
}

impl FileStringBundle {
    pub fn new<P: Into<PathBuf>, S: Into<String>>(path: P, string: S) -> Self {
        let path = FilePathComponent::construct(path.into());
        let string = FileStringComponent::construct(string.into());

        FileStringBundle { path, string }
    }
}

#[derive(hecs::Query)]
pub struct FileStringQuery<'a> {
    pub path: &'a FilePathComponent,
    pub string: &'a FileStringComponent,
}

#[derive(hecs::Query)]
pub struct FileBytesQuery<'a> {
    pub path: &'a FilePathComponent,
    pub string: &'a FileBytesComponent,
}

/// Load a file and store it in the World with a FileStringBundle
pub fn load_file_string<'a, 'b, P: Into<PathBuf>>(
    path: P,
) -> impl FnOnce(MessageContext<'a, 'b>) -> MessageResult<'a, 'b> {
    move |mut ctx| -> MessageResult<'a, 'b> {
        let (world, _) = &mut ctx;
        let path = path.into();

        println!(
            "Thread {} loading file {:?}...",
            std::thread::current().name().unwrap(),
            path,
        );
        let file = std::fs::read_to_string(&path)?;

        println!("Loaded file, spawning into world...");
        world.spawn(FileStringBundle::new(path, file));

        Ok(ctx)
    }
}

/// Load a file and store it in the World with a FileStringBundle
pub fn load_file_bytes<'a, 'b, P: Into<PathBuf>>(
    path: P,
) -> impl FnOnce(MessageContext<'a, 'b>) -> MessageResult<'a, 'b> {
    move |mut ctx| {
        let (world, _) = &mut ctx;
        let path = path.into();

        println!(
            "Thread {} loading file {:?}...",
            std::thread::current().name().unwrap(),
            path,
        );
        let file = std::fs::read_to_string(&path)?;

        println!("Loaded file, spawning into world...");
        world.spawn(FileStringBundle::new(path, file));

        Ok(ctx)
    }
}
