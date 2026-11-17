#define AppName "VietDict"
#define AppVersion "0.2.0"
#define AppPublisher "VietDict"
#define AppExeName "vietdict.exe"

[Setup]
AppId={{7F1F2E90-6A0B-4A2B-9D65-3C5AA5E6D5A1}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={autopf}\VietDict
DefaultGroupName=VietDict
OutputDir=..\..\dist
OutputBaseFilename=VietDict-windows-x64-setup
Compression=lzma2
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64
PrivilegesRequired=lowest
UninstallDisplayIcon={app}\{#AppExeName}
WizardStyle=modern

[Files]
Source: "..\..\dist\VietDict\vietdict.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\dist\VietDict\data\*"; DestDir: "{app}\data"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\..\README.md"; DestDir: "{app}"; Flags: isreadme
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\VietDict"; Filename: "{app}\{#AppExeName}"
Name: "{autodesktop}\VietDict"; Filename: "{app}\{#AppExeName}"

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Khởi chạy VietDict"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: filesandordirs; Name: "{app}\data"
