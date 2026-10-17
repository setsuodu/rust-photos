fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icon.ico");
        res.set("ProductName", "照片");
        res.set("FileDescription", "照片");
        if let Err(e) = res.compile() {
            println!("cargo:warning=embed icon failed: {e}");
        }
    }
}
