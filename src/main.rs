#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::hash_map::RandomState,
    fs,
    hash::{BuildHasher, Hasher},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{Fullscreen, WindowBuilder},
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use wry::WebViewBuilder;
#[cfg(windows)]
use wry::WebViewBuilderExtWindows;

mod video_edit;

const INDEX_HTML: &str = include_str!("index.html");
const APP_VERSION: &str = env!("APP_VERSION");

const IMAGE_EXTS: &[&str] = &[
    "jpg", "jpeg", "jfif", "png", "gif", "bmp", "webp", "ico", "svg", "avif",
];
const VIDEO_EXTS: &[&str] = &["mp4", "m4v", "webm", "mov", "mkv", "ogv"];
const PROGID: &str = "rust-photos.file";

#[derive(Clone, Copy, PartialEq)]
enum Kind { Image, Video }

impl Kind {
    fn as_str(self) -> &'static str {
        match self { Kind::Image => "image", Kind::Video => "video" }
    }
}

fn ext_of(p: &Path) -> String {
    p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

fn kind_of(p: &Path) -> Option<Kind> {
    let e = ext_of(p);
    if IMAGE_EXTS.contains(&e.as_str()) { Some(Kind::Image) }
    else if VIDEO_EXTS.contains(&e.as_str()) { Some(Kind::Video) }
    else { None }
}

fn mime_of(p: &Path) -> &'static str {
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
