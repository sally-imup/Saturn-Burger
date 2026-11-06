; Installer Saturn (Inno Setup 6).
; Dibangun otomatis oleh GitHub Actions; versi dikirim lewat /DMyAppVersion=...

#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif
#define MyAppName "Saturn"
#define MyAppExeName "saturn.exe"

[Setup]
; AppId harus SAMA di setiap rilis supaya installer baru memperbarui instalasi lama.
; Jangan diubah setelah rilis pertama.
AppId={{8F3C2B1A-6D4E-4F7A-9B2C-5A1E0D9C7B31}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher=Sally
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputDir=..\dist
OutputBaseFilename={#MyAppName}-{#MyAppVersion}-Setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Instal per-user (tanpa UAC). Pengguna tetap bisa memilih instal untuk semua user.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
; Tutup Saturn yang sedang berjalan saat update.
CloseApplications=yes
UninstallDisplayName={#MyAppName}
UninstallDisplayIcon={app}\{#MyAppExeName}
; Ikon installer dibuat otomatis oleh build.rs (cargo build) di target\icons\saturn.ico
SetupIconFile=..\target\icons\saturn.ico

[Tasks]
; Shortcut desktop dicentang secara default.
Name: "desktopicon"; Description: "Buat shortcut di desktop"; GroupDescription: "Shortcut:"

[Files]
Source: "..\dist\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Jalankan {#MyAppName}"; Flags: nowait postinstall skipifsilent
