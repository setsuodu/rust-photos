# rust-photos

用 Rust 实现的「Windows 照片」MVP：浏览图片、播放视频，在同一文件夹内翻页，Delete 删除。

## 构建与运行（Windows 10/11）

需要 Rust（stable）和 Microsoft Edge WebView2 Runtime（Win10/11 通常已自带）。

```powershell
cargo run --release -- "D:\Pictures\a.jpg"   # 打开某张图片/视频，并浏览它所在文件夹
cargo run --release                            # 空白页，点「打开」或 Ctrl+O
cargo build --release                          # 产物：target\release\rust-photos.exe
```

## 注册为默认打开方式

把 exe 放到固定位置后执行一次（写入**当前用户**注册表，一般不需要管理员）：

```powershell
.\rust-photos.exe --register
```

取消：

```powershell
.\rust-photos.exe --unregister
```

注册后双击 jpg/png/mp4 等会用本程序打开。若系统仍用旧程序，可：

- 右键文件 → **打开方式** → 选择「照片」
- 或 **设置 → 应用 → 默认应用** 里按扩展名指定

支持扩展名：jpg/jpeg/jfif/png/gif/bmp/webp/ico/svg/avif；mp4/m4v/webm/mov/mkv/ogv。

## 快捷键

| 按键 | 功能 |
| --- | --- |
| ← / → （PageUp / PageDown、鼠标滚轮） | 上一张 / 下一张（图片和视频混排） |
| Home / End | 第一张 / 最后一张 |
| Delete | 删除当前文件（移到回收站，可恢复），随后显示下一张 |
| Space | 视频播放 / 暂停 |
| Ctrl+O | 打开文件 |
| F / F11 / Esc | 全屏 / 退出全屏 |

## 实现说明

- `tao` + `wry`：原生窗口 + WebView2，图片与视频交给系统解码器。
- `tiny_http`：只监听 `127.0.0.1` 随机端口，URL 带一次性随机 token。
- `trash`：删除走回收站。`natord`：自然排序。
- `icon.ico` 嵌入 exe（任务栏/资源管理器图标）并用于窗口图标。
- `--register` 写入 `HKCU\Software\Classes`，出现在「打开方式」并尽量设为当前用户默认。

## 后续可做

HEIC/RAW 支持、缩略图胶片条、缩放/拖动/旋转、幻灯片播放、拖拽文件到窗口打开、删除前确认、监听文件夹变化。
