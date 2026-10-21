use std::{fs, path::{Path, PathBuf}};

pub const IMAGE_EXTS: &[&str] = &[
    "jpg", "jpeg", "jfif", "png", "gif", "bmp", "webp", "ico", "svg", "avif",
];
pub const VIDEO_EXTS: &[&str] = &["mp4", "m4v", "webm", "mov", "mkv", "ogv"];

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Image,
    Video,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Image => "image",
            Kind::Video => "video",
        }
    }
}

pub fn ext_of(p: &Path) -> String {
    p.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

pub fn kind_of(p: &Path) -> Option<Kind> {
    let e = ext_of(p);
    if IMAGE_EXTS.contains(&e.as_str()) {
        Some(Kind::Image)
    } else if VIDEO_EXTS.contains(&e.as_str()) {
        Some(Kind::Video)
    } else {
        None
    }
}

pub fn mime_of(p: &Path) -> &'static str {
    match ext_of(p).as_str() {
        "jpg" | "jpeg" | "jfif" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        "ogv" => "video/ogg",
        _ => "application/octet-stream",
    }
}

pub struct State {
    pub dir: PathBuf,
    pub files: Vec<PathBuf>,
    pub current: usize,
}

impl State {
    pub fn empty() -> Self {
        State {
            dir: PathBuf::new(),
            files: Vec::new(),
            current: 0,
        }
    }

    pub fn to_json(&self) -> String {
        let files: Vec<_> = self
            .files
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                    "kind": kind_of(p).map(Kind::as_str).unwrap_or("image"),
                })
            })
            .collect();
        serde_json::json!({
            "dir": self.dir.to_string_lossy(),
            "files": files,
            "current": self.current,
        })
        .to_string()
    }
}

pub fn load_path(path: &Path) -> Option<State> {
    let (dir, target) = if path.is_dir() {
        (path.to_path_buf(), None)
    } else {
        (path.parent()?.to_path_buf(), Some(path.to_path_buf()))
    };
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && kind_of(p).is_some())
        .collect();
    files.sort_by(|a, b| {
        let an = a
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let bn = b
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        natord::compare(&an, &bn)
    });
    let current = target
        .and_then(|t| files.iter().position(|f| *f == t))
        .unwrap_or(0);
    Some(State { dir, files, current })
}

pub fn pick_file_dialog() -> Option<PathBuf> {
    let mut all: Vec<&str> = IMAGE_EXTS.to_vec();
    all.extend_from_slice(VIDEO_EXTS);
    rfd::FileDialog::new()
        .set_title("打开图片或视频")
        .add_filter("图片和视频", &all)
        .add_filter("图片", IMAGE_EXTS)
        .add_filter("视频", VIDEO_EXTS)
        .pick_file()
}
