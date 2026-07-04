use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct FileDiscovery {
    include_set: GlobSet,
}

impl FileDiscovery {
    pub fn new(includes: &[String]) -> Result<Self, globset::Error> {
        let mut builder = GlobSetBuilder::new();
        for pat in includes {
            builder.add(Glob::new(pat)?);
        }
        Ok(Self {
            include_set: builder.build()?,
        })
    }

    pub fn discover(&self, root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let path = entry.path();
                let rel_path = path.strip_prefix(root).unwrap_or(path);
                if self.include_set.is_match(rel_path) {
                    files.push(path.to_path_buf());
                }
            }
        }
        files
    }
}

pub fn map_output_path(
    src_path: &Path,
    root: &Path,
    output_dir: &Path,
    ext: &str,
) -> Option<PathBuf> {
    let rel = src_path.strip_prefix(root).ok()?;
    let mut out = output_dir.to_path_buf();

    if let Some(parent) = rel.parent() {
        out.push(parent);
    }

    let file_stem = rel.file_stem()?.to_string_lossy();
    let name = if file_stem.starts_with('_') {
        &file_stem[1..]
    } else {
        &file_stem
    };

    out.push(format!("{}.{}", name, ext));
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_file_discovery() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("src/components")).unwrap();
        fs::write(root.join("src/a.scss"), "").unwrap();
        fs::write(root.join("src/components/_b.scss"), "").unwrap();
        fs::write(root.join("src/c.txt"), "").unwrap();

        let discovery = FileDiscovery::new(&["src/**/*.scss".to_string()]).unwrap();
        let mut files = discovery.discover(root);
        files.sort();

        assert_eq!(files.len(), 2);
        assert!(files[0].ends_with("src/a.scss"));
        assert!(files[1].ends_with("src/components/_b.scss"));
    }

    #[test]
    fn test_map_output_path() {
        let root = Path::new("/project");
        let out_dir = Path::new("/project/dist");

        let src1 = Path::new("/project/src/components/_button.scss");
        let out1 = map_output_path(src1, root, out_dir, "ts").unwrap();
        assert_eq!(out1, Path::new("/project/dist/src/components/button.ts"));

        let src2 = Path::new("/project/src/index.scss");
        let out2 = map_output_path(src2, root, out_dir, "js").unwrap();
        assert_eq!(out2, Path::new("/project/dist/src/index.js"));
    }
}
