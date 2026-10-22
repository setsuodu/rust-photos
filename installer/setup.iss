; Inno Setup - install and register image/video associations
#define MyAppName "照片"
#define MyAppExeName "rust-photos.exe"
#define MyAppPublisher "setsuodu"
#define MyAppURL "https://github.com/setsuodu/rust-photos"
#define MyAppProgIdImage "rust-photos.image"
#define MyAppProgIdVideo "rust-photos.video"
; 旧版单一 ProgID，安装时清理
#define MyAppProgIdLegacy "rust-photos.file"

#ifndef MyAppVersion
#define MyAppVersion "0.1.1"
#endif

[Setup]
AppId={{A8F3C2E1-9B4D-4F6A-8E2C-1D5B7A9C0E3F}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
DefaultDirName={localappdata}\rust-photos
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputDir=..\dist
OutputBaseFilename=rust-photos-setup-{#MyAppVersion}
SetupIconFile=..\icon.ico
UninstallDisplayIcon={app}\rust-photos.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
ChangesAssociations=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional options:"; Flags: unchecked
Name: "assoc"; Description: "Set as default app for images and videos"; GroupDescription: "Additional options:"; Flags: checkedonce

[Files]
Source: "..\target\x86_64-pc-windows-msvc\release\rust-photos.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\rust-photos.exe"
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\rust-photos.exe"; Tasks: desktopicon

[Registry]
; ---- 图片 ProgID ----
Root: HKCU; Subkey: "Software\Classes\{#MyAppProgIdImage}"; ValueType: string; ValueName: ""; ValueData: "图片文件"; Flags: uninsdeletekey; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\{#MyAppProgIdImage}"; ValueType: string; ValueName: "FriendlyTypeName"; ValueData: "图片文件"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\{#MyAppProgIdImage}\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\rust-photos.exe,0"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\{#MyAppProgIdImage}\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\rust-photos.exe"" ""%1"""; Tasks: assoc

; ---- 视频 ProgID ----
Root: HKCU; Subkey: "Software\Classes\{#MyAppProgIdVideo}"; ValueType: string; ValueName: ""; ValueData: "视频文件"; Flags: uninsdeletekey; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\{#MyAppProgIdVideo}"; ValueType: string; ValueName: "FriendlyTypeName"; ValueData: "视频文件"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\{#MyAppProgIdVideo}\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\rust-photos.exe,0"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\{#MyAppProgIdVideo}\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\rust-photos.exe"" ""%1"""; Tasks: assoc

; ---- 图片扩展名 ----
Root: HKCU; Subkey: "Software\Classes\.jpg"; ValueType: string; ValueName: ""; ValueData: "{#MyAppProgIdImage}"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.jpg\OpenWithProgids"; ValueType: string; ValueName: "{#MyAppProgIdImage}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.jpeg"; ValueType: string; ValueName: ""; ValueData: "{#MyAppProgIdImage}"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.jpeg\OpenWithProgids"; ValueType: string; ValueName: "{#MyAppProgIdImage}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.jfif"; ValueType: string; ValueName: ""; ValueData: "{#MyAppProgIdImage}"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.jfif\OpenWithProgids"; ValueType: string; ValueName: "{#MyAppProgIdImage}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.png"; ValueType: string; ValueName: ""; ValueData: "{#MyAppProgIdImage}"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.png\OpenWithProgids"; ValueType: string; ValueName: "{#MyAppProgIdImage}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.gif"; ValueType: string; ValueName: ""; ValueData: "{#MyAppProgIdImage}"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.gif\OpenWithProgids"; ValueType: string; ValueName: "{#MyAppProgIdImage}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.bmp"; ValueType: string; ValueName: ""; ValueData: "{#MyAppProgIdImage}"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.bmp\OpenWithProgids"; ValueType: string; ValueName: "{#MyAppProgIdImage}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.webp"; ValueType: string; ValueName: ""; ValueData: "{#MyAppProgIdImage}"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.webp\OpenWithProgids"; ValueType: string; ValueName: "{#MyAppProgIdImage}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.ico"; ValueType: string; ValueName: ""; ValueData: "{#MyAppProgIdImage}"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.ico\OpenWithProgids"; ValueType: string; ValueName: "{#MyAppProgIdImage}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.svg"; ValueType: string; ValueName: ""; ValueData: "{#MyAppProgIdImage}"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.svg\OpenWithProgids"; ValueType: string; ValueName: "{#MyAppProgIdImage}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.avif"; ValueType: string; ValueName: ""; ValueData: "{#MyAppProgIdImage}"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.avif\OpenWithProgids"; ValueType: string; ValueName: "{#MyAppProgIdImage}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc

; ---- 视频扩展名 ----
Root: HKCU; Subkey: "Software\Classes\.mp4"; ValueType: string; ValueName: ""; ValueData: "{#MyAppProgIdVideo}"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.mp4\OpenWithProgids"; ValueType: string; ValueName: "{#MyAppProgIdVideo}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.m4v"; ValueType: string; ValueName: ""; ValueData: "{#MyAppProgIdVideo}"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.m4v\OpenWithProgids"; ValueType: string; ValueName: "{#MyAppProgIdVideo}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.webm"; ValueType: string; ValueName: ""; ValueData: "{#MyAppProgIdVideo}"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.webm\OpenWithProgids"; ValueType: string; ValueName: "{#MyAppProgIdVideo}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.mov"; ValueType: string; ValueName: ""; ValueData: "{#MyAppProgIdVideo}"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.mov\OpenWithProgids"; ValueType: string; ValueName: "{#MyAppProgIdVideo}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.mkv"; ValueType: string; ValueName: ""; ValueData: "{#MyAppProgIdVideo}"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.mkv\OpenWithProgids"; ValueType: string; ValueName: "{#MyAppProgIdVideo}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.ogv"; ValueType: string; ValueName: ""; ValueData: "{#MyAppProgIdVideo}"; Tasks: assoc
Root: HKCU; Subkey: "Software\Classes\.ogv\OpenWithProgids"; ValueType: string; ValueName: "{#MyAppProgIdVideo}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc

; 清理旧版单一 ProgID（防止类型列仍显示「照片」）
Root: HKCU; Subkey: "Software\Classes\{#MyAppProgIdLegacy}"; Flags: deletekey; Tasks: assoc

[Run]
Filename: "{app}\rust-photos.exe"; Description: "Launch {#MyAppName}"; Flags: nowait postinstall skipifsilent
