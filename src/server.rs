use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    sync::Mutex,
};

use tiny_http::{Header, Method, Request, Response, StatusCode};

use crate::associations::{reveal_in_explorer, set_desktop_wallpaper, set_lock_screen_wallpaper};
use crate::state::{load_path, mime_of, pick_file_dialog, State};
use crate::util::{decode_base64, screenshots_dir};
use crate::video_edit;

const INDEX_HTML: &str = include_str!("index.html");
const APP_VERSION: &str = env!("APP_VERSION");

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

pub fn handle(mut req: Request, token: &str, state: &Mutex<State>) {
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
        (Method::Get, "ffmpeg-status") => {
            respond_json(
                req,
                200,
                serde_json::json!({ "available": video_edit::available() }).to_string(),
            );
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
        (Method::Post, "screenshot") => {
            let mut body = String::new();
            if req.as_reader().read_to_string(&mut body).is_err() {
                return respond_json(
                    req,
                    400,
                    serde_json::json!({ "error": "无法读取截图数据" }).to_string(),
                );
            }
            let body = body.trim();
            let (b64, file_stem) = if body.starts_with('{') {
                match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(v) => {
                        let data = v
                            .get("data")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string();
                        let name = v
                            .get("filename")
                            .and_then(|x| x.as_str())
                            .unwrap_or("截图")
                            .to_string();
                        (data, name)
                    }
                    Err(_) => {
                        return respond_json(
                            req,
                            400,
                            serde_json::json!({ "error": "截图数据无效" }).to_string(),
                        );
                    }
                }
            } else {
                let t = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                (body.to_string(), format!("Screenshot_{t}"))
            };
            let Some(png) = decode_base64(&b64) else {
                return respond_json(
                    req,
                    400,
                    serde_json::json!({ "error": "截图数据无效" }).to_string(),
                );
            };
            if png.is_empty() {
                return respond_json(
                    req,
                    400,
                    serde_json::json!({ "error": "截图为空" }).to_string(),
                );
            }
            let safe: String = file_stem
                .chars()
                .map(|c| match c {
                    '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                    _ => c,
                })
                .collect();
            let safe = safe.trim().trim_matches('.').to_string();
            let safe = if safe.is_empty() {
                "截图".to_string()
            } else {
                safe
            };
            let dir = screenshots_dir();
            let path = dir.join(format!("{safe}.png"));
            match fs::write(&path, &png) {
                Ok(()) => respond_json(
                    req,
                    200,
                    serde_json::json!({ "path": path.to_string_lossy() }).to_string(),
                ),
                Err(e) => respond_json(
                    req,
                    500,
                    serde_json::json!({ "error": format!("保存失败：{e}") }).to_string(),
                ),
            }
        }
        (Method::Post, r) if r.starts_with("reveal") => {
            let i = query_param(query, "i")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or_else(|| state.lock().unwrap().current);
            let path = {
                let s = state.lock().unwrap();
                s.files.get(i).cloned()
            };
            match path {
                Some(p) => {
                    if let Err(e) = reveal_in_explorer(&p) {
                        respond_json(req, 500, serde_json::json!({ "error": e }).to_string());
                    } else {
                        respond_json(req, 200, serde_json::json!({ "ok": true }).to_string());
                    }
                }
                None => respond_json(
                    req,
                    404,
                    serde_json::json!({ "error": "文件不存在" }).to_string(),
                ),
            }
        }
        (Method::Post, r) if r.starts_with("wallpaper") => {
            let i = query_param(query, "i")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or_else(|| state.lock().unwrap().current);
            let kind = query_param(query, "kind").unwrap_or("desktop");
            let path = {
                let s = state.lock().unwrap();
                s.files.get(i).cloned()
            };
            match path {
                Some(p) => {
                    let res = if kind == "lock" {
                        set_lock_screen_wallpaper(&p)
                    } else {
                        set_desktop_wallpaper(&p)
                    };
                    match res {
                        Ok(()) => respond_json(req, 200, serde_json::json!({ "ok": true }).to_string()),
                        Err(e) => respond_json(req, 500, serde_json::json!({ "error": e }).to_string()),
                    }
                }
                None => respond_json(
                    req,
                    404,
                    serde_json::json!({ "error": "文件不存在" }).to_string(),
                ),
            }
        }
        (Method::Post, "edit/trim") => {
            let mut body = String::new();
            if req.as_reader().read_to_string(&mut body).is_err() {
                return respond_json(
                    req,
                    400,
                    serde_json::json!({ "error": "无法读取参数" }).to_string(),
                );
            }
            let v: serde_json::Value = match serde_json::from_str(&body) {
                Ok(v) => v,
                Err(_) => {
                    return respond_json(
                        req,
                        400,
                        serde_json::json!({ "error": "请求参数无效" }).to_string(),
                    )
                }
            };
            let i = v
                .get("i")
                .and_then(|x| x.as_u64())
                .map(|n| n as usize)
                .unwrap_or_else(|| state.lock().unwrap().current);
            let start = v.get("start").and_then(|x| x.as_f64()).unwrap_or(0.0);
            let end = v.get("end").and_then(|x| x.as_f64());
            let duration = v.get("duration").and_then(|x| x.as_f64());
            let src = match state.lock().unwrap().files.get(i).cloned() {
                Some(p) => p,
                None => {
                    return respond_json(
                        req,
                        404,
                        serde_json::json!({ "error": "文件不存在" }).to_string(),
                    )
                }
            };
            match video_edit::trim(&src, start, end, duration) {
                Ok(out) => respond_json(
                    req,
                    200,
                    serde_json::json!({ "path": out.to_string_lossy() }).to_string(),
                ),
                Err(e) => respond_json(req, 500, serde_json::json!({ "error": e }).to_string()),
            }
        }
        (Method::Post, "edit/bitrate") => {
            let mut body = String::new();
            if req.as_reader().read_to_string(&mut body).is_err() {
                return respond_json(
                    req,
                    400,
                    serde_json::json!({ "error": "无法读取参数" }).to_string(),
                );
            }
            let v: serde_json::Value = match serde_json::from_str(&body) {
                Ok(v) => v,
                Err(_) => {
                    return respond_json(
                        req,
                        400,
                        serde_json::json!({ "error": "请求参数无效" }).to_string(),
                    )
                }
            };
            let i = v
                .get("i")
                .and_then(|x| x.as_u64())
                .map(|n| n as usize)
                .unwrap_or_else(|| state.lock().unwrap().current);
            let kbps = v
                .get("bitrate")
                .or_else(|| v.get("kbps"))
                .and_then(|x| x.as_i64())
                .unwrap_or(3000);
            let src = match state.lock().unwrap().files.get(i).cloned() {
                Some(p) => p,
                None => {
                    return respond_json(
                        req,
                        404,
                        serde_json::json!({ "error": "文件不存在" }).to_string(),
                    )
                }
            };
            match video_edit::bitrate(&src, kbps) {
                Ok(out) => respond_json(
                    req,
                    200,
                    serde_json::json!({ "path": out.to_string_lossy() }).to_string(),
                ),
                Err(e) => respond_json(req, 500, serde_json::json!({ "error": e }).to_string()),
            }
        }
        (Method::Post, "edit/split") => {
            let mut body = String::new();
            if req.as_reader().read_to_string(&mut body).is_err() {
                return respond_json(
                    req,
                    400,
                    serde_json::json!({ "error": "无法读取参数" }).to_string(),
                );
            }
            let v: serde_json::Value = match serde_json::from_str(&body) {
                Ok(v) => v,
                Err(_) => {
                    return respond_json(
                        req,
                        400,
                        serde_json::json!({ "error": "请求参数无效" }).to_string(),
                    )
                }
            };
            let i = v
                .get("i")
                .and_then(|x| x.as_u64())
                .map(|n| n as usize)
                .unwrap_or_else(|| state.lock().unwrap().current);
            let side = v.get("side").and_then(|x| x.as_str()).unwrap_or("both");
            let src = match state.lock().unwrap().files.get(i).cloned() {
                Some(p) => p,
                None => {
                    return respond_json(
                        req,
                        404,
                        serde_json::json!({ "error": "文件不存在" }).to_string(),
                    )
                }
            };
            match video_edit::split_h(&src, side) {
                Ok(outs) => {
                    let paths: Vec<_> = outs
                        .iter()
                        .map(|p| p.to_string_lossy().into_owned())
                        .collect();
                    respond_json(
                        req,
                        200,
                        serde_json::json!({ "paths": paths, "path": paths.first() }).to_string(),
                    );
                }
                Err(e) => respond_json(req, 500, serde_json::json!({ "error": e }).to_string()),
            }
        }
        (Method::Post, "edit/aspect") => {
            let mut body = String::new();
            if req.as_reader().read_to_string(&mut body).is_err() {
                return respond_json(
                    req,
                    400,
                    serde_json::json!({ "error": "无法读取参数" }).to_string(),
                );
            }
            let v: serde_json::Value = match serde_json::from_str(&body) {
                Ok(v) => v,
                Err(_) => {
                    return respond_json(
                        req,
                        400,
                        serde_json::json!({ "error": "请求参数无效" }).to_string(),
                    )
                }
            };
            let i = v
                .get("i")
                .and_then(|x| x.as_u64())
                .map(|n| n as usize)
                .unwrap_or_else(|| state.lock().unwrap().current);
            let ratio = v.get("ratio").and_then(|x| x.as_str()).unwrap_or("16:9");
            let mode = v.get("mode").and_then(|x| x.as_str()).unwrap_or("stretch");
            let src = match state.lock().unwrap().files.get(i).cloned() {
                Some(p) => p,
                None => {
                    return respond_json(
                        req,
                        404,
                        serde_json::json!({ "error": "文件不存在" }).to_string(),
                    )
                }
            };
            match video_edit::change_aspect(&src, ratio, mode) {
                Ok(out) => respond_json(
                    req,
                    200,
                    serde_json::json!({ "path": out.to_string_lossy() }).to_string(),
                ),
                Err(e) => respond_json(req, 500, serde_json::json!({ "error": e }).to_string()),
            }
        }
        _ => respond_status(req, 404),
    }
}
