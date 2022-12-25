use std::{
    ffi::OsStr,
    fs::{self, DirEntry, File},
    io::Write,
    path::Path,
};

use eyre::{ensure, Context};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Repository {
    name: String,
    description: String,
}

impl Repository {
    pub(crate) fn create(&self, project_root: impl AsRef<Path>) -> eyre::Result<()> {
        let project_root = project_root.as_ref();
        if self
            .name
            .split('/')
            .any(|stem| stem.ends_with(".git") || !is_valid_repo_stem(stem))
        {
            eyre::bail!("invalid repository name: {}", self.name);
        }

        let path = project_root.join(format!("{}.git", self.name));
        git2::Repository::init_opts(
            &path,
            git2::RepositoryInitOptions::new()
                .bare(true)
                .no_reinit(true)
                .no_dotgit_dir(true)
                .mkpath(true)
                .description(&self.description),
        )
        .wrap_err_with(|| format!("failed to create repository at {}", path.display()))?;

        // workaround for https://github.com/rust-lang/git2-rs/issues/848
        //   git2::RepositoryInitOptions::description is ignored
        let desc_path = path.join("description");
        File::options()
            .write(true)
            .truncate(true)
            .open(&desc_path)
            .wrap_err_with(|| format!("failed to open description file: {}", desc_path.display()))?
            .write_all(self.description.as_bytes())
            .wrap_err_with(|| {
                format!(
                    "failed to write to description file: {}",
                    desc_path.display()
                )
            })?;

        Ok(())
    }

    fn read(project_root: impl AsRef<Path>, path: impl AsRef<Path>) -> eyre::Result<Self> {
        let project_root = project_root.as_ref();
        let path = path.as_ref();
        let desc_path = path.join("description");

        let _ = git2::Repository::open(path)
            .wrap_err_with(|| format!("invalid git repository: {}", path.display()))?;
        let name = path
            .strip_prefix(project_root)
            .wrap_err_with(|| format!("cannot strip project_root: {}", path.display()))?
            .with_extension("") // trim .git
            .to_string_lossy()
            .to_string();
        let description = fs::read_to_string(desc_path).unwrap_or_default();

        Ok(Self { name, description })
    }
}

// https://github.com/dead-claudia/github-limits#repository-names
fn is_valid_repo_stem(s: impl AsRef<OsStr>) -> bool {
    let s = s.as_ref().to_str().unwrap_or("");
    !s.is_empty()
        && s != "."
        && s != ".."
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

pub(crate) fn iter(project_root: &Path) -> eyre::Result<Iter> {
    let read_dir = project_root
        .read_dir()
        .wrap_err_with(|| format!("failed to read directory: {}", project_root.display()))?;
    let read_dir = vec![read_dir];
    Ok(Iter {
        project_root,
        read_dir,
    })
}

#[derive(Debug)]
pub(crate) struct Iter<'a> {
    project_root: &'a Path,
    read_dir: Vec<fs::ReadDir>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = eyre::Result<Repository>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let read_dir = self.read_dir.last_mut()?;
            match read_dir.next() {
                Some(entry) => {
                    let res = entry
                        .wrap_err("failed to read directory entry")
                        .and_then(|entry| read_dir_entry(self.project_root, entry));
                    match res {
                        Ok(ReadDirEntry::Repository(repo)) => return Some(Ok(repo)),
                        Ok(ReadDirEntry::Directory(dir)) => self.read_dir.push(dir),
                        Err(e) => return Some(Err(e)),
                    }
                }
                None => {
                    let _ = self.read_dir.pop();
                }
            };
        }
    }
}

#[derive(Debug)]
enum ReadDirEntry {
    Repository(Repository),
    Directory(fs::ReadDir),
}

fn read_dir_entry(project_root: &Path, entry: DirEntry) -> eyre::Result<ReadDirEntry> {
    let path = entry.path();
    let file_name = entry.file_name();

    ensure!(path.is_dir(), "not a directory: {}", path.display());
    ensure!(!path.is_symlink(), "path is symlink: {}", path.display());
    ensure!(
        is_valid_repo_stem(file_name),
        "invalid repository name: {}",
        path.display()
    );

    if path.extension() != Some("git".as_ref()) {
        let read_dir = fs::read_dir(&path)
            .wrap_err_with(|| format!("failed to read directory: {}", path.display()))?;
        return Ok(ReadDirEntry::Directory(read_dir));
    }

    let repo = Repository::read(project_root, path)?;
    Ok(ReadDirEntry::Repository(repo))
}
