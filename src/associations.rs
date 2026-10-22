use std::path::Path;

use crate::state::{IMAGE_EXTS, VIDEO_EXTS};

/// 图片与视频分开注册，避免资源管理器「类型」列全部显示同一个名字。
pub const PROGID_IMAGE: &str = "rust-photos.image";
pub const PROGID_VIDEO: &str = "rust-photos.video";
/// 旧版单 ProgID，卸载时一并清理。
const PROGID_LEGACY: &str = "rust-photos.file";

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
fn register_progid(
    classes: &winreg::RegKey,
    progid: &str,
    friendly: &str,
    exe: &str,
) -> Result<(), String> {
    use winreg::enums::*;
    let (prog, _) = classes.create_subkey(progid).map_err(|e| e.to_string())?;
    prog.set_value("", &friendly).map_err(|e| e.to_string())?;
    // FriendlyTypeName 优先被资源管理器使用（Windows 2000+）
    prog.set_value("FriendlyTypeName", &friendly)
        .map_err(|e| e.to_string())?;
    let (cmd, _) = prog
        .create_subkey("shell\\open\\command")
        .map_err(|e| e.to_string())?;
    cmd.set_value("", &format!("\"{exe}\" \"%1\""))
        .map_err(|e| e.to_string())?;
    let (icon, _) = prog.create_subkey("DefaultIcon").map_err(|e| e.to_string())?;
    icon.set_value("", &format!("{exe},0"))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(windows)]
fn associate_exts(
    classes: &winreg::RegKey,
    exts: &[&str],
    progid: &str,
) -> Result<(), String> {
    for ext in exts {
        let key_name = format!(".{ext}");
        let (ext_key, _) = classes.create_subkey(&key_name).map_err(|e| e.to_string())?;
        ext_key.set_value("", &progid).map_err(|e| e.to_string())?;
        let (owp, _) = ext_key
            .create_subkey("OpenWithProgids")
            .map_err(|e| e.to_string())?;
        owp.set_value(progid, &"").map_err(|e| e.to_string())?;
    }
    Ok(())
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

    register_progid(&classes, PROGID_IMAGE, "图片文件", &exe)?;
    register_progid(&classes, PROGID_VIDEO, "视频文件", &exe)?;

    associate_exts(&classes, IMAGE_EXTS, PROGID_IMAGE)?;
    associate_exts(&classes, VIDEO_EXTS, PROGID_VIDEO)?;

    // 清理旧版单一 ProgID，避免残留导致类型仍显示「照片」
    let _ = classes.delete_subkey_all(PROGID_LEGACY);

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

    let all_progids = [PROGID_IMAGE, PROGID_VIDEO, PROGID_LEGACY];

    for ext in IMAGE_EXTS.iter().chain(VIDEO_EXTS.iter()) {
        if let Ok(ext_key) =
            classes.open_subkey_with_flags(format!(".{ext}"), KEY_READ | KEY_WRITE)
        {
            if let Ok(val) = ext_key.get_value::<String, _>("") {
                if all_progids.contains(&val.as_str()) {
                    let _ = ext_key.delete_value("");
                }
            }
            if let Ok(owp) = ext_key.open_subkey_with_flags("OpenWithProgids", KEY_WRITE) {
                for pid in &all_progids {
                    let _ = owp.delete_value(pid);
                }
            }
        }
    }

    for pid in &all_progids {
        let _ = classes.delete_subkey_all(pid);
    }

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
