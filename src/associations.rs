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

/// 设为桌面壁纸（SystemParametersInfo，当前用户，无需管理员）
#[cfg(windows)]
pub fn set_desktop_wallpaper(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let display = match std::fs::canonicalize(path) {
        Ok(abs) => dunce_path(&abs),
        Err(_) => dunce_path(path),
    };
    if !Path::new(&display).exists() {
        return Err(format!("文件不存在：{display}"));
    }
    let wide: Vec<u16> = std::ffi::OsStr::new(&display)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    #[link(name = "user32")]
    unsafe extern "system" {
        fn SystemParametersInfoW(
            action: u32,
            param: u32,
            pv: *const u16,
            winini: u32,
        ) -> i32;
    }
    const SPI_SETDESKWALLPAPER: u32 = 0x0014;
    const SPIF_UPDATEINIFILE: u32 = 0x01;
    const SPIF_SENDCHANGE: u32 = 0x02;
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            wide.as_ptr(),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        )
    };
    if ok == 0 {
        return Err("SystemParametersInfo 设置桌面壁纸失败".into());
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn set_desktop_wallpaper(path: &Path) -> Result<(), String> {
    let _ = path;
    Err("仅支持 Windows".into())
}

/// 设为锁屏壁纸（优先 WinRT 当前用户；失败时尝试 PersonalizationCSP，后者通常需管理员）
#[cfg(windows)]
pub fn set_lock_screen_wallpaper(path: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let display = match std::fs::canonicalize(path) {
        Ok(abs) => dunce_path(&abs),
        Err(_) => dunce_path(path),
    };
    if !Path::new(&display).exists() {
        return Err(format!("文件不存在：{display}"));
    }

    // 1) 尝试 PowerShell + WinRT（多数 Win10/11 当前用户可用，无需管理员）
    let ps = format!(
        r#"$ErrorActionPreference='Stop'; $p = '{}'; Add-Type -AssemblyName System.Runtime.WindowsRuntime; $asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | ? {{ $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' }})[0]; Function Await($WinRtTask, $ResultType) {{ $asTask = $asTaskGeneric.MakeGenericMethod($ResultType); $netTask = $asTask.Invoke($null, @($WinRtTask)); $netTask.Wait(-1) | Out-Null; $netTask.Result }}; $null = [Windows.Storage.StorageFile,Windows.Storage,ContentType=WindowsRuntime]; $file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($p)) ([Windows.Storage.StorageFile]); $null = [Windows.System.UserProfile.LockScreen,Windows.System.UserProfile,ContentType=WindowsRuntime]; [Windows.System.UserProfile.LockScreen]::SetImageFileAsync($file).AsTask().Wait()"#,
        display.replace('\'', "''")
    );
    let status = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &ps])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .status();
    if let Ok(s) = status {
        if s.success() {
            return Ok(());
        }
    }

    // 2) 回退：PersonalizationCSP（HKLM，常需管理员）
    use winreg::enums::*;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    match hklm.create_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\PersonalizationCSP") {
        Ok((key, _)) => {
            key.set_value("LockScreenImagePath", &display)
                .map_err(|e| e.to_string())?;
            key.set_value("LockScreenImageUrl", &display)
                .map_err(|e| e.to_string())?;
            key.set_value("LockScreenImageStatus", &1u32)
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        Err(e) => Err(format!(
            "WinRT 设置锁屏失败，且写入 PersonalizationCSP 需要管理员权限：{e}"
        )),
    }
}

#[cfg(not(windows))]
pub fn set_lock_screen_wallpaper(path: &Path) -> Result<(), String> {
    let _ = path;
    Err("仅支持 Windows".into())
}
