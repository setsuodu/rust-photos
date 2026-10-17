# rust-photos

用 Rust 实现的「Windows 照片」MVP：浏览图片、播放视频，在同一文件夹内翻页，Delete 删除。

## 构建与运行（Windows 10/11）

需要 Rust（stable）和 Microsoft Edge WebView2 Runtime（Win10/11 通常已自带）。

```powershell
cargo run --release -- "D:\Pictures\a.jpg"   # 打开某张图片/视频，并浏览它所在文件夹
cargo run --release                            # 不带参数：弹出"打开文件"对话框
cargo build --release                          # 产物：target\release\rust-photos.exe
```

想要双击图片就用它打开：右键图片 →「打开方式」→「选择其他应用」→ 选 `rust-photos.exe`。

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

- `tao` + `wry`：原生窗口 + WebView2，图片与视频交给系统解码器，所以支持 H.264 mp4 等系统能播的格式。
- `tiny_http`：只监听 `127.0.0.1` 随机端口，URL 带一次性随机 token；负责页面、Range 视频流、文件夹扫描、删除、打开对话框。
- `trash`：删除走回收站。`natord`：与资源管理器一致的自然排序（img2 在 img10 之前）。
- 支持格式：jpg/jpeg/jfif/png/gif/bmp/webp/ico/svg/avif；mp4/m4v/webm/mov/mkv/ogv（mkv、mov 能否播放取决于编码和系统解码器）。

## 后续可做

HEIC/RAW 支持、缩略图胶片条、缩放/拖动/旋转、幻灯片播放、拖拽文件到窗口打开、删除前确认、监听文件夹变化。
