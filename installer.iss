; ULTRON MARK LIV - Complete Desktop Application Installer Script
; Inno Setup 6 Script

#define MyAppName "ULTRON"
#define MyAppVersion "1.58.0"
#define MyAppPublisher "Nibir"
#define MyAppExeName "ULTRON.exe"

[Setup]
AppId={{C15904D7-A103-4D93-9F9B-9838183EBF41}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={localappdata}\Programs\ULTRON
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
OutputDir=dist
OutputBaseFilename=ultron_nibir
SetupIconFile=config\jarvis.ico
Compression=lzma2/ultra64
SolidCompression=yes
PrivilegesRequired=lowest
WizardStyle=modern
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\config\jarvis.ico

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "dist\ULTRON\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\config\jarvis.ico"
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\config\jarvis.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
var
  UserNamePage: TInputQueryWizardPage;

procedure InitializeWizard;
begin
  UserNamePage := CreateInputQueryPage(
    wpSelectDir,
    'Personalize ULTRON',
    'Who will ULTRON be serving?',
    'Please enter your name below. ULTRON will address you respectfully (e.g., "Zakiul sir" or "Nibir sir"):'
  );
  UserNamePage.Add('Your Name (Boss / User):', False);
  UserNamePage.Values[0] := 'Nibir';
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  ConfigFile: String;
  ConfigLines: TArrayOfString;
  UserName: String;
  I: Integer;
  Line: String;
begin
  if CurStep = ssPostInstall then
  begin
    UserName := Trim(UserNamePage.Values[0]);
    if UserName = '' then
      UserName := 'Nibir';

    StringChangeEx(UserName, '"', '', True);
    StringChangeEx(UserName, '\', '', True);

    ConfigFile := ExpandConstant('{app}\config\api_keys.json');
    if FileExists(ConfigFile) then
    begin
      if LoadStringsFromFile(ConfigFile, ConfigLines) then
      begin
        for I := 0 to GetArrayLength(ConfigLines) - 1 do
        begin
          Line := ConfigLines[I];
          if Pos('"user_name"', Line) > 0 then
          begin
            ConfigLines[I] := '    "user_name": "' + UserName + '",';
          end;
        end;
        SaveStringsToFile(ConfigFile, ConfigLines, False);
      end;
    end;
  end;
end;
