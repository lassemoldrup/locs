use std::fs::{File, ReadDir, DirEntry, read_dir};
use std::path::{Path, PathBuf};
use std::io::{Result, BufReader, BufRead, Write, stdout};
use std::time::Instant;
use clap::{AppSettings, Clap};

#[derive(Clap)]
#[clap(version = env!("CARGO_PKG_VERSION"), author = "Lasse Møldrup <lasse.moeldrup@gmail.com>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Args {
    #[clap(default_value = "./", about = "Sets the path(s) to search")]
    paths: Vec<PathBuf>,
    #[clap(short, long, about = "Sets specific file extensions to search")]
    extensions: Option<Vec<String>>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let extensions = args.extensions.as_ref();

    let stdout = stdout();
    let mut handle = stdout.lock();

    let mut total = 0;
    let start = Instant::now();
    for file_info in FileTraverser::traverse(&args.paths, extensions)? {
        let file_info = file_info?;

        let loc = BufReader::new(file_info.file)
            .lines()
            .count();
        total += loc;

        writeln!(handle, "{}\t{}", file_info.path.display(), loc)?;
    }
    let elapsed = start.elapsed().as_millis();
    writeln!(handle, "Total: {}. Completed in {} ms.", total, elapsed)
}


#[derive(Debug)]
struct FileTraverser<'a, T> {
    extensions: Option<&'a Vec<T>>,
    sub_dirs: Vec<PathBuf>,
    traverser: ReadDir,
}

impl<'a, T: AsRef<str>> FileTraverser<'a, T> {
    fn traverse(starting_dirs: &[PathBuf], extensions: Option<&'a Vec<T>>) -> Result<Self> {
        let sub_dirs = starting_dirs[1..].to_vec();
        let traverser = read_dir(&starting_dirs[0])?;
        Ok(Self {
            extensions,
            sub_dirs,
            traverser,
        })
    }

    fn map_entry(&mut self, entry: Result<DirEntry>) -> Result<Option<FileInfo>> {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_file() {
            match self.extensions {
                Some(exts) => if has_ext(&entry, exts) {
                    return File::open(&path)
                        .map(|file| Some(FileInfo::new(file, &path)));
                },
                None => return File::open(&path)
                    .map(|file| Some(FileInfo::new(file, &path))),
            }
        } else if file_type.is_dir() {
            self.sub_dirs.push(path);
        }

        Ok(None)
    }
}

fn has_ext(entry: &DirEntry, exts: &[impl AsRef<str>]) -> bool {
    exts.iter().any(|ext| entry.file_name()
        .to_string_lossy()
        .ends_with(ext.as_ref()))
}

impl<'a, T: AsRef<str>> Iterator for FileTraverser<'a, T> {
    type Item = Result<FileInfo>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = match self.traverser.next() {
                Some(entry) => entry,
                None => {
                    self.traverser = match read_dir(self.sub_dirs.pop()?) {
                        Ok(traverser) => traverser,
                        Err(err) => return Some(Err(err)),
                    };
                    continue;
                }
            };

            let file_info = self.map_entry(entry).transpose();
            if file_info.is_some() {
                return file_info;
            }
        }
    }
}


#[derive(Debug)]
struct FileInfo {
    file: File,
    path: PathBuf,
}

impl FileInfo {
    fn new(file: File, path: &Path) -> Self {
        let path = PathBuf::from(path);
        Self {
            file,
            path,
        }
    }
}
