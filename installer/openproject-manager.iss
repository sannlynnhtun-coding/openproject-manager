[Setup]
AppId={{99EEB178-F840-4F32-A61C-C7404DFA6715}
AppName=OpenProject Manager
AppVersion=1.0.0
DefaultDirName={autopf}\OpenProject Manager
DefaultGroupName=OpenProject Manager
OutputBaseFilename=OpenProjectManager-Setup
MinVersion=10.0.14393
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=admin
DisableDirPage=yes
DisableProgramGroupPage=yes
Compression=lzma2/normal
SolidCompression=yes
SetupIconFile=..\assets\openproject.ico
UninstallDisplayIcon={app}\openproject-manager.exe
AppComments=Native Windows Ruby, PostgreSQL and OpenProject. No Docker or WSL required.

[Files]
Source: "{#ManagerSource}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BundleSource}\setup-native.ps1"; DestDir: "{app}"; Flags: ignoreversion uninsneveruninstall
Source: "{#BundleSource}\service.rb"; DestDir: "{app}"; Flags: ignoreversion uninsneveruninstall
Source: "{#BundleSource}\openproject\config\initializers\windows_file_store.rb"; DestDir: "{app}\openproject\config\initializers"; Flags: ignoreversion uninsneveruninstall
Source: "{#BundleSource}\openproject\app\views\layouts\_common_head.html.erb"; DestDir: "{app}\openproject\app\views\layouts"; Flags: ignoreversion uninsneveruninstall
Source: "{#BundleSource}\openproject\public\styles\boardui.css"; DestDir: "{app}\openproject\public\styles"; Flags: ignoreversion uninsneveruninstall
Source: "{#BundleSource}\*"; DestDir: "{app}"; Excludes: "\setup-native.ps1,\service.rb,\openproject\tmp\*,\openproject\log\*,\openproject\files\*"; Flags: recursesubdirs createallsubdirs onlyifdoesntexist uninsneveruninstall

[Dirs]
Name: "{app}\openproject\tmp"; Flags: uninsneveruninstall
Name: "{app}\openproject\log"; Flags: uninsneveruninstall

[Icons]
Name: "{group}\OpenProject Manager"; Filename: "{app}\openproject-manager.exe"
Name: "{autodesktop}\OpenProject Manager"; Filename: "{app}\openproject-manager.exe"

[Run]
Filename: "{app}\openproject-manager.exe"; Description: "Open OpenProject Manager"; Flags: postinstall nowait skipifsilent runascurrentuser

[UninstallRun]
Filename: "{app}\openproject-manager.exe"; Parameters: "--remove-services"; Flags: runhidden; RunOnceId: RemoveOpenProjectServices; Check: RemoveServices

[Code]
var
  RemoveServiceChoice: Boolean;

procedure CurStepChanged(CurStep: TSetupStep);
var
  ExitCode: Integer;
  Detail: AnsiString;
begin
  if CurStep = ssPostInstall then
  begin
    WizardForm.StatusLabel.Caption := 'Configuring private PostgreSQL and OpenProject services. First setup takes several minutes...';
    if not Exec(ExpandConstant('{app}\openproject-manager.exe'), '--setup-bundled', '', SW_HIDE, ewWaitUntilTerminated, ExitCode) then
      RaiseException('Could not start OpenProject setup. Existing data was preserved.');
    if ExitCode <> 0 then
    begin
      LoadStringFromFile(ExpandConstant('{commonappdata}\OpenProjectManager\installer.log'), Detail);
      RaiseException('OpenProject setup failed: ' + String(Detail) + #13#10 + 'See %PROGRAMDATA%\OpenProjectManager\logs\setup.log. Resolve the error and run setup again; existing data is retained.');
    end;
  end;
end;

function InitializeUninstall(): Boolean;
begin
  RemoveServiceChoice := False;
  if not UninstallSilent then
    RemoveServiceChoice := MsgBox('Also stop and remove the OpenProject Windows services? Database, attachments, credentials and runtime files will be kept.', mbConfirmation, MB_YESNO) = IDYES;
  Result := True;
end;

function RemoveServices(): Boolean;
begin
  Result := RemoveServiceChoice;
end;
