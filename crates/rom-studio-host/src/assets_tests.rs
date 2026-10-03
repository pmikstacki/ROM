use super::assets::Assets;
use std::path::PathBuf;

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "rom-studio-assets-{}-{}",
            std::process::id(),
            super::session::secret().unwrap()
        ));
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        std::fs::write(dir.join("index.html"), "<html>studio</html>").unwrap();
        std::fs::write(dir.join("assets/main.js"), "export const app=true;").unwrap();
        Self(dir)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn navigation_fallback_does_not_serve_unknown_assets_or_traversal() {
    let dir = Directory::new();
    let assets = Assets::load(&dir.0, 16, 1024).unwrap();
    assert_eq!(
        assets.get("resources/tasks").unwrap().bytes.as_ref(),
        b"<html>studio</html>"
    );
    assert!(assets.get("assets/missing.js").is_none());
    assert!(assets.get("../index.html").is_none());
    assert!(assets.get("assets/%2fmain.js").is_none());
    assert!(assets.get("assets//main.js").is_none());
    assert!(assets.get("/etc/passwd").is_none());
    assert_eq!(
        assets.get("assets/main.js").unwrap().content_type,
        "text/javascript; charset=utf-8"
    );
}

#[test]
fn asset_inventory_bounds_are_enforced_before_serving() {
    let dir = Directory::new();
    assert!(Assets::load(&dir.0, 1, 1024).is_err());
    assert!(Assets::load(&dir.0, 16, 8).is_err());
    std::fs::remove_file(dir.0.join("index.html")).unwrap();
    assert!(Assets::load(&dir.0, 16, 1024).is_err());
}

#[cfg(unix)]
#[test]
fn inventory_refuses_symbolic_links_instead_of_following_them() {
    let dir = Directory::new();
    std::os::unix::fs::symlink("/etc/passwd", dir.0.join("assets/secret.txt")).unwrap();
    assert!(Assets::load(&dir.0, 16, 1024).is_err());
}

#[test]
fn empty_directory_trees_cannot_bypass_inventory_work_limits() {
    let dir = Directory::new();
    let mut path = dir.0.clone();
    for _ in 0..40 {
        path = path.join("nested");
        std::fs::create_dir(&path).unwrap();
    }
    assert!(Assets::load(&dir.0, 16, 1024).is_err());
}
