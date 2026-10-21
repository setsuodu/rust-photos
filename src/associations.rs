use std::path::Path;

use crate::state::{IMAGE_EXTS, VIDEO_EXTS};

pub const PROGID: &str = "rust-photos.file";

#[cfg(windows)]
fn dunce_path(p: &Path) -> String {
    dunce::simplified(p).to_string_lossy().replace('/', "\\")
}

#[cfg(windows)]
fn notify_shell() {
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SHChangeNotify(event: i32, flags: u32, item1: usize, item2: usize);
    }
    unsafe {
        SHChangeNotify(0x0800_0000, 0, 0, 0);
    }
}

#[cfg(windows)]
pub fn register_associations() -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = dunce_path(&exe);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu
        .open_subkey_with_flags("Software\\Classes", KEY_WRITE)
        .map_err(|e| format!("无法写入注册表: {e}"))?;
    let (prog, _) = classes.create_subkey(PROGID).map_err(|e| e.to_string())?;
    prog.set_value("", &"照片").map_err(|e| e.to_string())?;
    let (cmd, _) = prog
        .create_subkey("shell\\open\\command")
        .map_err(|e| e.to_string())?;
    cmd.set_value("", &format!("\"{exe}\" \"%1\""))
        .map_err(|e| e.to_string())?;
    let (icon, _) = prog.create_subkey("DefaultIcon").map_err(|e| e.to_string())?;
    icon.set_value("", &format!("{exe},0"))
        .map_err(|e| e.to_string())?;
    for ext in IMAGE_EXTS.iter().chain(VIDEO_EXTS.iter()) {
        let key_name = format!(".{ext}");
        let (ext_key, _) = classes.create_subkey(&key_name).map_err(|e| e.to_string())?;
        ext_key.set_value("", &PROGID).map_err(|e| e.to_string())?;
        let (owp, _) = ext_key
            .create_subkey("OpenWithProgids")
            .map_err(|e| e.to_string())?;
        owp.set_value(PROGID, &"").map_err(|e| e.to_string())?;
    }
    notify_shell();
    Ok(())
}

#[cfg(windows)]
pub fn unregister_associations() -> Result<(), String> {
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
            if let Ok(owp) = ext_key.open_subkey_with_flags("OpenWithProgids", KEY_WRITE) {
                let _ = owp.delete_value(PROGID);
            }
        }
    }
    let _ = classes.delete_subkey_all(PROGID);
    notify_shell();
    Ok(())
}

#[cfg(not(windows))]
pub fn register_associations() -> Result<(), String> {
    Err("仅支持 Windows".into())
}

#[cfg(not(windows))]
pub fn unregister_associations() -> Result<(), String> {
    Err("仅支持 Windows".into())
}

#[cfg(windows)]
pub fn reveal_in_explorer(path: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let display = match std::fs::canonicalize(path) {
        Ok(abs) => dunce_path(&abs),
        Err(_) => dunce_path(path),
    };
    if !Path::new(&display).exists() && !path.exists() {
        return Err(format!("文件不存在：{display}"));
    }
    let arg = format!("/select,\"{display}\"");
    std::process::Command::new("explorer.exe")
        .raw_arg(arg)
        .spawn()
        .map_err(|e| format!("无法打开资源管理器：{e}"))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn reveal_in_explorer(path: &Path) -> Result<(), String> {
    let _ = path;
    Err("仅支持 Windows".into())
}
