; Inno Setup script for T1-Customizer (free edition) on Windows 10/11.
; Builds an installer with start-menu entry, optional logon autostart and a
; clean uninstaller.

#define AppName "T1-Customizer"
#define AppNameJa "T1-Customizer"
#define AppVersion "1.0.6"
#define AppPublisher "AppWizz Kai"
#define AppURL "https://github.com/drkai-lab/T1-Customizer"

[Setup]
; Do not change: shared with 2.1.0 so Windows sees one app.
AppId={{B7E2C1A0-3F4D-4E5A-9B8C-1D2E3F4A5B6C}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
VersionInfoVersion=1.0.6.0
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
AppUpdatesURL={#AppURL}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppNameJa}
DisableProgramGroupPage=yes
UsePreviousAppDir=no
UsePreviousGroup=no
CloseApplications=yes
OutputDir=..\dist-win
OutputBaseFilename=t1-customizer-{#AppVersion}-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
SetupIconFile=..\icons\app.ico
UninstallDisplayIcon={app}\t1-keyboard-config-gui.exe
ArchitecturesInstallIn64BitMode=x64compatible
ArchitecturesAllowed=x64compatible
; {autopf} is Program Files, so the default install is administrative. The
; dialog lets a non-admin choose a per-user install instead of elevating
; as someone else. HKCU autostart is not written from [Registry]: an elevated
; token's HKCU is not the person who launched setup. reg.exe runs as that
; person (runasoriginaluser). The reg.exe call runs as that person (runasoriginaluser).
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=dialog

[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
japanese.AutostartTask=ログオン時に起動する
english.AutostartTask=Start at logon
japanese.AutostartGroup=自動起動
english.AutostartGroup=Autostart

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "autostart"; Description: "{cm:AutostartTask}"; GroupDescription: "{cm:AutostartGroup}"; Flags: unchecked

[Files]
Source: "..\dist-win\t1-keyboard-config.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist-win\t1-keyboard-config-gui.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\icons\*"; DestDir: "{app}\icons"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppNameJa}"; Filename: "{app}\t1-keyboard-config-gui.exe"
Name: "{group}\Uninstall {#AppNameJa}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppNameJa}"; Filename: "{app}\t1-keyboard-config-gui.exe"; Tasks: desktopicon

[Run]
Filename: "{sys}\reg.exe"; Parameters: "add ""HKCU\Software\Microsoft\Windows\CurrentVersion\Run"" /v ""T1-Customizer"" /t REG_SZ /d ""\""{app}\t1-keyboard-config-gui.exe\"""" /f"; Flags: runhidden runasoriginaluser; Tasks: autostart
Filename: "{app}\t1-keyboard-config-gui.exe"; Description: "{cm:LaunchProgram,{#AppNameJa}}"; Flags: nowait postinstall skipifsilent runasoriginaluser

[Code]
procedure DeleteRunValue(const Name: String);
var
  ResultCode: Integer;
begin
  ExecAsOriginalUser(
    ExpandConstant('{sys}\reg.exe'),
    'delete "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v "' + Name + '" /f',
    '',
    SW_HIDE,
    ewWaitUntilTerminated,
    ResultCode);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  { The CI compiler rejects runasoriginaluser in the uninstall-run section.
    Delete the Run values as the person who launched uninstall. }
  if CurUninstallStep = usUninstall then
  begin
    DeleteRunValue('T1-Customizer');
  end;
end;
