fn main() {
    // CI 打 tag 发版时设置 APP_VERSION（去掉 v 前缀的 tag），本地开发回退到 Cargo.toml
    let ver = std::env::var("APP_VERSION")
        .or_else(|_| std::env::var("CARGO_PKG_VERSION"))
        .unwrap_or_else(|_| "0.0.0-dev".into());
    println!("cargo:rustc-env=APP_VERSION={ver}");
    println!("cargo:rerun-if-env-changed=APP_VERSION");

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
