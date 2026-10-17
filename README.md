# rust-photos

用 Rust 实现的「Windows 照片」MVP：浏览图片、播放视频，同文件夹翻页，Delete 删到回收站。

## 安装（推荐）

到 [Releases](https://github.com/setsuodu/rust-photos/releases) 下载 **`rust-photos-setup-x.x.x.exe`**，双击安装。

安装向导里默认勾选 **「设为图片和视频的默认打开方式」**，装完即可用。

卸载走「应用和功能」或开始菜单里的卸载项，关联会一并清掉。

## 开发构建

```powershell
cargo build --release --target x86_64-pc-windows-msvc
# 可选：本机装了 Inno Setup 6 时打安装包
iscc /DMyAppVersion=0.1.1 installer\setup.iss
```

```powershell
cargo run --release -- "D:\Pictures\a.jpg"   # 打开指定文件
cargo run --release                            # 空白页，点「打开」
```

## 快捷键

| 按键 | 功能 |
| --- | --- |
| ← / → / 滚轮 | 上一张 / 下一张 |
| Home / End | 第一张 / 最后一张 |
| Delete | 移到回收站 |
| Space | 视频播放 / 暂停 |
| Ctrl+O | 打开文件 |
| F / F11 / Esc | 全屏 |

## 说明

- WebView2 渲染图片/视频（系统解码器）
- 支持：jpg/png/gif/bmp/webp/ico/svg/avif；mp4/webm/mov/mkv 等
- 绿色便携包也可在 Release 里下 zip；需要关联时用安装包更省事
