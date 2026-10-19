use std::io::{Read, Write};
use std::path::PathBuf;

fn main() {
    let ver = std::env::var("APP_VERSION")
        .or_else(|_| std::env::var("CARGO_PKG_VERSION"))
        .unwrap_or_else(|_| "0.0.0-dev".into());
    println!("cargo:rustc-env=APP_VERSION={ver}");
    println!("cargo:rerun-if-env-changed=APP_VERSION");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("restored_main.rs");

    let b64 = [
        include_str!("src/main.rs.gz.b64.0"),
        include_str!("src/main.rs.gz.b64.1"),
        include_str!("src/main.rs.gz.b64.2"),
        include_str!("src/main.rs.gz.b64.3"),
    ]
    .concat();
    let gz = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        b64.trim(),
    )
    .expect("base64 decode main.rs.gz.b64.*");
    let mut dec = flate2::read::GzDecoder::new(&gz[..]);
    let mut src = String::new();
    dec.read_to_string(&mut src).expect("gunzip main.rs");
    assert!(src.contains("fn main()"), "restored main missing fn main");
    assert!(src.len() > 20_000, "restored main too short");

    // Strip crate-level inner attribute (lives in src/main.rs now)
    src = src
        .lines()
        .filter(|l| !l.starts_with("#![cfg_attr"))
        .collect::<Vec<_>>()
        .join("\n");

    // Fix paths that were relative to src/ when this was a normal main.rs
    let video_path = format!("{manifest_dir}/src/video_edit.rs").replace('\\', "/");
    let index_path = format!("{manifest_dir}/src/index.html").replace('\\', "/");
    let icon_path = format!("{manifest_dir}/icon.ico").replace('\\', "/");

    src = src.replace(
        "mod video_edit;",
        &format!("#[path = \"{video_path}\"]\nmod video_edit;"),
    );
    src = src.replace(
        "include_str!(\"index.html\")",
        &format!("include_str!(\"{index_path}\")"),
    );
    src = src.replace(
        "include_bytes!(\"../icon.ico\")",
        &format!("include_bytes!(\"{icon_path}\")"),
    );

    std::fs::File::create(&out)
        .expect("create restored_main.rs")
        .write_all(src.as_bytes())
        .expect("write restored_main.rs");

    println!("cargo:rerun-if-changed=src/main.rs.gz.b64.0");
    println!("cargo:rerun-if-changed=src/main.rs.gz.b64.1");
    println!("cargo:rerun-if-changed=src/main.rs.gz.b64.2");
    println!("cargo:rerun-if-changed=src/main.rs.gz.b64.3");
    println!("cargo:rerun-if-changed=src/video_edit.rs");
    println!("cargo:rerun-if-changed=src/index.html");
    println!("cargo:rerun-if-changed=icon.ico");

    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icon.ico");
        res.set("ProductName", "照片");
        res.set("FileDescription", "照片");
        res.set("FileVersion", &ver);
        res.set("ProductVersion", &ver);
        if let Err(e) = res.compile() {
            println!("cargo:warning=embed icon failed: {e}");
        }
    }
}
