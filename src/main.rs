//! Rust 版「Windows 照片」MVP
//!
//! 架构：
//! - tao + wry：原生窗口 + WebView2 负责渲染图片、播放视频（直接用系统解码器）
//! - tiny_http：只监听 127.0.0.1 的本地小服务，负责
//!     * 提供页面
//!     * 以 Range 方式流式提供图片/视频（视频拖动进度条需要）
//!     * 文件夹扫描、删除（进回收站）、打开文件对话框
//! - 所有请求带一次性随机 token，避免本机浏览器里的其他网页误调用删除接口
//!
//! 命令行：
//!   rust-photos.exe [文件或文件夹]   打开指定内容
//!   rust-photos.exe                  显示空白页（点「打开」或 Ctrl+O）
//!   rust-photos.exe --register       注册为图片/视频的打开方式（当前用户）
//!   rust-photos.exe --unregister     取消注册

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
    window::WindowBuilder,
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use wry::WebViewBuilder;
#[cfg(windows)]
use wry::WebViewBuilderExtWindows;

const INDEX_HTML: &str = include_str!("index.html");
/// 与 Cargo.toml / CI tag 同步的客户端版本号
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

const IMAGE_EXTS: &[&str] = &[
    "jpg", "jpeg", "jfif", "png", "gif", "bmp", "webp", "ico", "svg", "avif",
];
const VIDEO_EXTS: &[&str] = &["mp4", "m4v", "webm", "mov", "mkv", "ogv"];

const PROGID: &str = "rust-photos.file";

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Image,
    Video,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Image => "image",
            Kind::Video => "video",
        }
    }
}

fn ext_of(p: &Path) -> String {
    p.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

fn kind_of(p: &Path) -> Option<Kind> {
    let e = ext_of(p);
    if IMAGE_EXTS.contains(&e.as_str()) {
        Some(Kind::Image)
    } else if VIDEO_EXTS.contains(&e.as_str()) {
        Some(Kind::Video)
    } else {
        None
    }
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

/// 当前浏览的文件夹、文件列表与当前位置
struct State {
    dir: PathBuf,
    files: Vec<PathBuf>,
    current: usize,
}

impl State {
    fn empty() -> Self {
        State {
            dir: PathBuf::new(),
            files: Vec::new(),
            current: 0,
        }
    }

    fn to_json(&self) -> String {
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

/// 打开一个文件（或文件夹）：扫描所在文件夹的全部图片/视频，并定位到该文件
fn load_path(path: &Path) -> Option<State> {
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

    // 与资源管理器一致的"自然排序"：img2 排在 img10 之前，不区分大小写
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

    Some(State {
        dir,
        files,
        current,
    })
}

fn pick_file_dialog() -> Option<PathBuf> {
    let mut all: Vec<&str> = IMAGE_EXTS.to_vec();
    all.extend_from_slice(VIDEO_EXTS);
    rfd::FileDialog::new()
        .set_title("打开图片或视频")
        .add_filter("图片和视频", &all)
        .add_filter("图片", IMAGE_EXTS)
        .add_filter("视频", VIDEO_EXTS)
        .pick_file()
}

fn load_window_icon() -> Option<tao::window::Icon> {
    let bytes = include_bytes!("../icon.ico");
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    tao::window::Icon::from_rgba(img.into_raw(), w, h).ok()
}

// ---------------------------------------------------------------- 文件关联 ----

#[cfg(windows)]
fn register_associations() -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = dunce_path(&exe);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu
        .open_subkey_with_flags("Software\\Classes", KEY_WRITE)
        .map_err(|e| format!("无法写入注册表: {e}"))?;

    // ProgID
    let (prog, _) = classes
        .create_subkey(PROGID)
        .map_err(|e| e.to_string())?;
    prog.set_value("", &"照片")
        .map_err(|e| e.to_string())?;
    let (cmd, _) = prog
        .create_subkey("shell\\open\\command")
        .map_err(|e| e.to_string())?;
    cmd.set_value("", &format!("\"{exe}\" \"%1\""))
        .map_err(|e| e.to_string())?;
    let (icon, _) = prog
        .create_subkey("DefaultIcon")
        .map_err(|e| e.to_string())?;
    icon.set_value("", &format!("{exe},0"))
        .map_err(|e| e.to_string())?;

    for ext in IMAGE_EXTS.iter().chain(VIDEO_EXTS.iter()) {
        let key_name = format!(".{ext}");
        let (ext_key, _) = classes
            .create_subkey(&key_name)
            .map_err(|e| e.to_string())?;
        // 设为当前用户默认打开方式
        ext_key
            .set_value("", &PROGID)
            .map_err(|e| e.to_string())?;
        // 同时挂到「打开方式」列表
        let (owp, _) = ext_key
            .create_subkey("OpenWithProgids")
            .map_err(|e| e.to_string())?;
        owp.set_value(PROGID, &"")
            .map_err(|e| e.to_string())?;
    }

    // 通知资源管理器刷新图标/关联
    notify_shell();
    Ok(())
}

#[cfg(windows)]
fn unregister_associations() -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu
        .open_subkey_with_flags("Software\\Classes", KEY_WRITE)
        .map_err(|e| format!("无法写入注册表: {e}"))?;

    for ext in IMAGE_EXTS.iter().chain(VIDEO_EXTS.iter()) {
        if let Ok(ext_key) =
            classes.open_subkey_with_flags(format!(".{ext}"), KEY_READ | KEY_WRITE)
        {
            if let Ok(val) = ext_key.get_value::<String, _>("") {
                if val == PROGID {
                    let _ = ext_key.delete_value("");
                }
            }
            if let Ok(owp) =
                ext_key.open_subkey_with_flags("OpenWithProgids", KEY_WRITE)
            {
                let _ = owp.delete_value(PROGID);
            }
        }
    }
    let _ = classes.delete_subkey_all(PROGID);
    notify_shell();
    Ok(())
}

#[cfg(windows)]
fn dunce_path(p: &Path) -> String {
    // 去掉 \\?\ 前缀，注册表命令行更稳妥
    let s = p.to_string_lossy();
    let s = s.strip_prefix(r"\\?\\").unwrap_or(&s);
    s.replace('/', "\\")
}

#[cfg(windows)]
fn notify_shell() {
    // SHCNE_ASSOCCHANGED = 0x08000000, SHCNF_IDLIST = 0
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SHChangeNotify(event: i32, flags: u32, item1: usize, item2: usize);
    }
    unsafe {
        SHChangeNotify(0x0800_0000, 0, 0, 0);
    }
}

#[cfg(not(windows))]
fn register_associations() -> Result<(), String> {
    Err("仅支持 Windows".into())
}

#[cfg(not(windows))]
fn unregister_associations() -> Result<(), String> {
    Err("仅支持 Windows".into())
}

// ---------------------------------------------------------------- HTTP ----

fn hdr(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).unwrap()
}

fn respond_json(req: Request, status: u16, body: String) {
    let resp = Response::from_string(body)
        .with_status_code(StatusCode(status))
        .with_header(hdr("Content-Type", "application/json; charset=utf-8"))
        .with_header(hdr("Cache-Control", "no-store"));
    let _ = req.respond(resp);
}

fn respond_status(req: Request, status: u16) {
    let _ = req.respond(Response::empty(StatusCode(status)));
}

/// 解析 `Range: bytes=a-b`，返回闭区间 (start, end)
fn parse_range(value: &str, total: u64) -> Option<(u64, u64)> {
    let spec = value.trim().strip_prefix("bytes=")?;
    let spec = spec.split(',').next()?.trim();
    let (a, b) = spec.split_once('-')?;
    if total == 0 {
        return None;
    }
    if a.is_empty() {
        let n: u64 = b.parse().ok()?;
        if n == 0 {
            return None;
        }
        Some((total.saturating_sub(n), total - 1))
    } else {
        let start: u64 = a.parse().ok()?;
        let end: u64 = if b.is_empty() {
            total - 1
        } else {
            b.parse().ok()?
        };
        if start > end || start >= total {
            return None;
        }
        Some((start, end.min(total - 1)))
    }
}

fn serve_media(req: Request, path: PathBuf) {
    let mut file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return respond_status(req, 404),
    };
    let total = file.metadata().map(|m| m.len()).unwrap_or(0);

    let range_header = req
        .headers()
        .iter()
        .find(|h| h.field.equiv("Range"))
        .map(|h| h.value.as_str().to_string());

    let (start, end, partial) = match range_header {
        Some(r) => match parse_range(&r, total) {
            Some((s, e)) => (s, e, true),
            None => {
                let resp = Response::empty(StatusCode(416))
                    .with_header(hdr("Content-Range", &format!("bytes */{total}")));
                let _ = req.respond(resp);
                return;
            }
        },
        None => (0, total.saturating_sub(1), false),
    };

    let len = if total == 0 { 0 } else { end - start + 1 };
    if file.seek(SeekFrom::Start(start)).is_err() {
        return respond_status(req, 500);
    }

    let mut headers = vec![
        hdr("Content-Type", mime_of(&path)),
        hdr("Accept-Ranges", "bytes"),
        hdr("Cache-Control", "no-store"),
    ];
    if partial {
        headers.push(hdr(
            "Content-Range",
            &format!("bytes {start}-{end}/{total}"),
        ));
    }
    let status = if partial { 206 } else { 200 };
    let resp = Response::new(
        StatusCode(status),
        headers,
        file.take(len),
        Some(len as usize),
        None,
    );
    let _ = req.respond(resp);
}

fn query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then_some(v)
    })
}

fn handle(req: Request, token: &str, state: &Mutex<State>) {
    let url = req.url().to_string();
    let (path, query) = url.split_once('?').unwrap_or((url.as_str(), ""));

    let prefix = format!("/{token}/");
    let Some(rest) = path.strip_prefix(prefix.as_str()) else {
        return respond_status(req, 403);
    };
    let method = req.method().clone();

    match (method, rest) {
        (Method::Get, "") => {
            let html = INDEX_HTML.replace("__APP_VERSION__", APP_VERSION);
            let resp = Response::from_string(html)
                .with_header(hdr("Content-Type", "text/html; charset=utf-8"))
                .with_header(hdr("Cache-Control", "no-store"));
            let _ = req.respond(resp);
        }

        (Method::Get, "state") => {
            let body = state.lock().unwrap().to_json();
            respond_json(req, 200, body);
        }

        (Method::Get, r) if r.starts_with("media/") => {
            let id: Option<usize> = r["media/".len()..].parse().ok();
            let path = id.and_then(|i| state.lock().unwrap().files.get(i).cloned());
            match path {
                Some(p) => serve_media(req, p),
                None => respond_status(req, 404),
            }
        }

        (Method::Post, "delete") => {
            let Some(i) = query_param(query, "i").and_then(|v| v.parse::<usize>().ok()) else {
                return respond_status(req, 400);
            };
            let mut s = state.lock().unwrap();
            if i >= s.files.len() {
                return respond_json(
                    req,
                    404,
                    serde_json::json!({ "error": "文件不存在" }).to_string(),
                );
            }
            match trash::delete(&s.files[i]) {
                Ok(()) => {
                    s.files.remove(i);
                    s.current = if s.files.is_empty() {
                        0
                    } else {
                        i.min(s.files.len() - 1)
                    };
                    let body = s.to_json();
                    drop(s);
                    respond_json(req, 200, body);
                }
                Err(e) => {
                    drop(s);
                    respond_json(
                        req,
                        500,
                        serde_json::json!({ "error": format!("删除失败：{e}") }).to_string(),
                    );
                }
            }
        }

        (Method::Post, "open") => {
            if let Some(p) = pick_file_dialog() {
                if let Some(ns) = load_path(&p) {
                    *state.lock().unwrap() = ns;
                }
            }
            let body = state.lock().unwrap().to_json();
            respond_json(req, 200, body);
        }

        _ => respond_status(req, 404),
    }
}

fn random_token() -> String {
    let a = RandomState::new().build_hasher().finish();
    let b = RandomState::new().build_hasher().finish();
    format!("{a:016x}{b:016x}")
}

fn show_console_message(msg: &str) {
    // windows_subsystem=windows 时没有控制台，用 MessageBox 提示注册结果
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "user32")]
        unsafe extern "system" {
            fn MessageBoxW(hwnd: isize, text: *const u16, caption: *const u16, ty: u32) -> i32;
        }
        let to_wide = |s: &str| {
            std::ffi::OsStr::new(s)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect::<Vec<u16>>()
        };
        let text = to_wide(msg);
        let caption = to_wide("照片");
        unsafe {
            MessageBoxW(0, text.as_ptr(), caption.as_ptr(), 0x40);
        }
    }
    #[cfg(not(windows))]
    {
        eprintln!("{msg}");
    }
}

// ---------------------------------------------------------------- main ----

fn main() {
    let mut args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();

    if let Some(first) = args.first().and_then(|a| a.to_str()) {
        match first {
            "--register" | "/register" => {
                match register_associations() {
                    Ok(()) => show_console_message(
                        "已注册为图片与视频的打开方式（当前用户）。\n\n\
若资源管理器仍用旧程序打开，可右键文件 → 打开方式 → 选择「照片」，\
或到「设置 → 应用 → 默认应用」里指定。",
                    ),
                    Err(e) => show_console_message(&format!("注册失败：{e}")),
                }
                return;
            }
            "--unregister" | "/unregister" => {
                match unregister_associations() {
                    Ok(()) => show_console_message("已取消文件关联注册。"),
                    Err(e) => show_console_message(&format!("取消注册失败：{e}")),
                }
                return;
            }
            _ => {}
        }
    }

    // 用法：rust-photos.exe [文件或文件夹]
    // 不带参数时进入空白页（点「打开」或 Ctrl+O），不再自动弹对话框
    let initial = match args.first().map(PathBuf::from) {
        Some(p) => load_path(&p).unwrap_or_else(State::empty),
        None => State::empty(),
    };
    let _ = args;

    let state = Arc::new(Mutex::new(initial));
    let token = random_token();

    let server = Server::http("127.0.0.1:0").expect("无法启动本地服务");
    let port = server.server_addr().to_ip().expect("无效地址").port();

    {
        let state = Arc::clone(&state);
        let token = token.clone();
        thread::spawn(move || {
            for req in server.incoming_requests() {
                let state = Arc::clone(&state);
                let token = token.clone();
                thread::spawn(move || handle(req, &token, &state));
            }
        });
    }

    let event_loop = EventLoopBuilder::<String>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let mut wb = WindowBuilder::new()
        .with_title("照片")
        .with_inner_size(LogicalSize::new(1200.0, 800.0));
    if let Some(icon) = load_window_icon() {
        wb = wb.with_window_icon(Some(icon));
    }
    let window = wb.build(&event_loop).expect("无法创建窗口");

    let data_dir = std::env::var_os("LOCALAPPDATA")
        .map(|p| PathBuf::from(p).join("rust-photos").join("webview2"));
    let mut web_context = wry::WebContext::new(data_dir);

    let builder = WebViewBuilder::new_with_web_context(&mut web_context)
        .with_url(format!("http://127.0.0.1:{port}/{token}/"))
        .with_ipc_handler(move |req| {
            if let Some(t) = req.body().strip_prefix("title:") {
                let _ = proxy.send_event(t.to_string());
            }
        });

    #[cfg(windows)]
    let builder = builder
        .with_additional_browser_args(
            "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
             --autoplay-policy=no-user-gesture-required",
        )
        .with_browser_accelerator_keys(false);

    let _webview = builder.build(&window).expect("无法创建 WebView");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::UserEvent(title) => window.set_title(&title),
            _ => {}
        }
    });
}
