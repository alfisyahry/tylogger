; Inno Setup Script for Tylogger (Universal Windows Log Analysis Tool)
; Compatible with Windows 7 SP1 through Windows 11 (x86 & x64)

#define MyAppName "Tylogger"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "Tylogger Contributors"
#define MyAppURL "https://github.com/LogExperts/LogExpert"
#define MyAppExeName "tylogger.exe"
#define MyAppMutex "TyloggerSingleInstanceMutex"

[Setup]
; Unique AppId for upgrade detection
AppId={{D41A28A7-873B-49E2-9BC4-28EFB0C40F51}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
OutputDir=..\dist
OutputBaseFilename=Tylogger-{#MyAppVersion}-windows-setup
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern

; Dual Installation Modes (Per-User default, All-Users elevated option)
; PrivilegesRequired=lowest allows non-admin installation into %LOCALAPPDATA%\Programs
; PrivilegesRequiredOverridesAllowed enables UAC prompt when installing for all users
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog commandline

; Windows Version Requirement: Windows 7 SP1 or newer
MinVersion=6.1sp1

; Architecture handling (Defaults to 64-bit on x64 systems, 32-bit on x86)
ArchitecturesInstallIn64BitMode=x64compatible
ArchitecturesAllowed=x86 x64compatible

; Single Instance & Running Application Detection (RSK-31, AG-40)
AppMutex={#MyAppMutex}

; Visual & UI Contract Compliance (Native, Clean, Professional)
DisableProgramGroupPage=yes
DisableWelcomePage=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "fileassoc_session"; Description: "Associate Tylogger session files (.tysession, .tyworkspace)"; GroupDescription: "File Associations:"
Name: "contextmenu"; Description: "Add 'Open with Tylogger' to Explorer context menu for logs"; GroupDescription: "Shell Integration:"; Flags: unchecked

[Files]
; 64-bit binary component
Source: "..\dist\universal-x64\tylogger.exe"; DestDir: "{app}"; Check: Is64BitInstallMode; Flags: ignoreversion
; 32-bit binary component (fallback / 32-bit OS)
Source: "..\dist\universal-x86\tylogger.exe"; DestDir: "{app}"; Check: not Is64BitInstallMode; Flags: ignoreversion

; Documentation and licenses
Source: "..\README.txt"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\THIRD_PARTY_NOTICES"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist

; Sample files
Source: "..\examples\*"; DestDir: "{app}\examples"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist

[Dirs]
Name: "{app}\config"; Permissions: users-modify
Name: "{app}\plugins"; Permissions: users-modify

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
; Document Associations (.tysession)
Root: HKA; Subkey: "Software\Classes\.tysession"; ValueType: string; ValueName: ""; ValueData: "Tylogger.Session"; Flags: uninsdeletevalue; Tasks: fileassoc_session
Root: HKA; Subkey: "Software\Classes\Tylogger.Session"; ValueType: string; ValueName: ""; ValueData: "Tylogger Session File"; Flags: uninsdeletekey; Tasks: fileassoc_session
Root: HKA; Subkey: "Software\Classes\Tylogger.Session\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Flags: uninsdeletekey; Tasks: fileassoc_session
Root: HKA; Subkey: "Software\Classes\Tylogger.Session\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Flags: uninsdeletekey; Tasks: fileassoc_session

; Document Associations (.tyworkspace)
Root: HKA; Subkey: "Software\Classes\.tyworkspace"; ValueType: string; ValueName: ""; ValueData: "Tylogger.Workspace"; Flags: uninsdeletevalue; Tasks: fileassoc_session
Root: HKA; Subkey: "Software\Classes\Tylogger.Workspace"; ValueType: string; ValueName: ""; ValueData: "Tylogger Workspace File"; Flags: uninsdeletekey; Tasks: fileassoc_session
Root: HKA; Subkey: "Software\Classes\Tylogger.Workspace\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Flags: uninsdeletekey; Tasks: fileassoc_session
Root: HKA; Subkey: "Software\Classes\Tylogger.Workspace\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Flags: uninsdeletekey; Tasks: fileassoc_session

; Explorer Context Menu ("Open with Tylogger" for text and log files - non-intrusive)
Root: HKA; Subkey: "Software\Classes\SystemFileAssociations\.log\shell\OpenWithTylogger"; ValueType: string; ValueName: ""; ValueData: "Open with Tylogger"; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKA; Subkey: "Software\Classes\SystemFileAssociations\.log\shell\OpenWithTylogger\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKA; Subkey: "Software\Classes\SystemFileAssociations\.txt\shell\OpenWithTylogger"; ValueType: string; ValueName: ""; ValueData: "Open with Tylogger"; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKA; Subkey: "Software\Classes\SystemFileAssociations\.txt\shell\OpenWithTylogger\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Flags: uninsdeletekey; Tasks: contextmenu

[Run]
Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Filename: "{app}\{#MyAppExeName}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; Only delete files installed in application directory; preserve user logs and %APPDATA%\Tylogger (RSK-30)
Type: files; Name: "{app}\portable.flag"
Type: dirifempty; Name: "{app}\config"
Type: dirifempty; Name: "{app}\plugins"
Type: dirifempty; Name: "{app}\examples"
Type: dirifempty; Name: "{app}"

[Code]
// Helper function to ensure user log files and user profiles are preserved during uninstall
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
  begin
    // Note: %APPDATA%\Tylogger preferences and session files are intentionally NOT deleted
    // to protect user data across upgrades and uninstalls (RSK-30).
  end;
end;
