; Installeur Windows de Turtlefin (Inno Setup 6).
;
; Compilation : iscc /DVersion=0.9.0 /DArch=x64 /DSrc=<dossier des fichiers> /DOut=<dossier de sortie> turtlefin.iss
;   Arch : x64 ou x86. Src contient turtlefin.exe, libmpv-2.dll et les autres fichiers à installer.
;
; Deux modes, choisis sur une page de l'assistant :
; - Installation : dans Program Files (ou un dossier choisi), menu Démarrer, désinstallation ;
; - Portable : les fichiers seuls, dans le dossier choisi (clé USB...), avec un fichier « portable »
;   qui fait garder la configuration, le cache et les téléchargements dans ce dossier.
; La mise à jour intégrée relance cet installeur en silence (/SILENT /DIR=...) : le mode d'origine
; est retrouvé (fichier « portable » présent ou non dans le dossier).

#ifndef Version
  #define Version "0.0.0"
#endif
#ifndef Arch
  #define Arch "x64"
#endif
#ifndef Src
  #define Src "..\..\target\release"
#endif
#ifndef Out
  #define Out "."
#endif

[Setup]
AppId={{6D3E2F9A-6C1B-4E7B-9C1A-54B8E7F0A1C2}
AppName=Turtlefin
AppVersion={#Version}
AppVerName=Turtlefin {#Version}
AppPublisher=Xelopteryx
AppPublisherURL=https://github.com/Xelopteryx/Turtlefin
AppSupportURL=https://github.com/Xelopteryx/Turtlefin/issues
DefaultDirName={autopf}\Turtlefin
DefaultGroupName=Turtlefin
DisableProgramGroupPage=yes
; Choix du dossier toujours proposé (même pour une mise à jour, sauf en mode silencieux).
DisableDirPage=no
UsePreviousAppDir=yes
; Installation pour l'utilisateur seul (pas de droits administrateur). Pour tous : /ALLUSERS.
; (« dialog » afficherait ce choix avant celui de la langue, qui doit venir en premier.)
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=commandline
OutputDir={#Out}
OutputBaseFilename=Turtlefin-{#Version}-windows-{#Arch}-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
SetupIconFile=..\icons\turtlefin.ico
; Logo dans l'assistant (100 % et 200 %), images produites par packaging/icons/make-icons.py.
WizardImageFile=wizard-large-1x.bmp,wizard-large-2x.bmp
WizardSmallImageFile=wizard-small-1x.bmp,wizard-small-2x.bmp
; Langue de l'assistant proposée d'après celle de Windows ; elle devient celle de Turtlefin (fichier « language »).
ShowLanguageDialog=yes
LanguageDetectionMethod=uilanguage
CloseApplications=force
RestartApplications=no
UninstallDisplayName=Turtlefin
UninstallDisplayIcon={app}\turtlefin.exe
; Pas de désinstalleur pour une copie portable.
Uninstallable=not IsPortable
CreateUninstallRegKey=not IsPortable
#if Arch == "x64"
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
#endif

[Languages]
Name: "fr"; MessagesFile: "compiler:Languages\French.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "de"; MessagesFile: "compiler:Languages\German.isl"
Name: "it"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "pt"; MessagesFile: "compiler:Languages\Portuguese.isl"
Name: "pl"; MessagesFile: "compiler:Languages\Polish.isl"
Name: "nl"; MessagesFile: "compiler:Languages\Dutch.isl"

[CustomMessages]
fr.ModeTitle=Type d'installation
fr.ModeSub=Comment veux-tu utiliser Turtlefin ?
fr.ModeText=Installation : Turtlefin s'installe sur cet ordinateur (menu Démarrer, désinstallation dans les paramètres de Windows).%nPortable : les fichiers seuls, dans le dossier de ton choix (une clé USB par exemple) ; la configuration et les téléchargements restent dans ce dossier.
fr.ModeInstall=Installation (recommandé)
fr.ModePortable=Portable
en.ModeTitle=Installation type
en.ModeSub=How do you want to use Turtlefin?
en.ModeText=Install: Turtlefin is installed on this computer (Start menu, uninstall from Windows settings).%nPortable: just the files, in the folder of your choice (a USB stick for example); settings and downloads stay in that folder.
en.ModeInstall=Install (recommended)
en.ModePortable=Portable
es.ModeTitle=Tipo de instalación
es.ModeSub=¿Cómo quieres usar Turtlefin?
es.ModeText=Instalación: Turtlefin se instala en este equipo (menú Inicio, desinstalación desde la configuración de Windows).%nPortátil: solo los archivos, en la carpeta que elijas (una memoria USB, por ejemplo); la configuración y las descargas se quedan en esa carpeta.
es.ModeInstall=Instalación (recomendado)
es.ModePortable=Portátil
de.ModeTitle=Installationsart
de.ModeSub=Wie möchtest du Turtlefin verwenden?
de.ModeText=Installation: Turtlefin wird auf diesem Computer installiert (Startmenü, Deinstallation über die Windows-Einstellungen).%nPortabel: nur die Dateien, in einem Ordner deiner Wahl (zum Beispiel ein USB-Stick); Einstellungen und Downloads bleiben in diesem Ordner.
de.ModeInstall=Installation (empfohlen)
de.ModePortable=Portabel
it.ModeTitle=Tipo di installazione
it.ModeSub=Come vuoi usare Turtlefin?
it.ModeText=Installazione: Turtlefin viene installato su questo computer (menu Start, disinstallazione dalle impostazioni di Windows).%nPortatile: solo i file, nella cartella che scegli (una chiavetta USB, per esempio); impostazioni e download restano in quella cartella.
it.ModeInstall=Installazione (consigliato)
it.ModePortable=Portatile
pt.ModeTitle=Tipo de instalação
pt.ModeSub=Como queres usar o Turtlefin?
pt.ModeText=Instalação: o Turtlefin é instalado neste computador (menu Iniciar, desinstalação nas definições do Windows).%nPortátil: apenas os ficheiros, na pasta que escolheres (uma pen USB, por exemplo); as definições e as transferências ficam nessa pasta.
pt.ModeInstall=Instalação (recomendado)
pt.ModePortable=Portátil
pl.ModeTitle=Rodzaj instalacji
pl.ModeSub=Jak chcesz używać Turtlefin?
pl.ModeText=Instalacja: Turtlefin zostanie zainstalowany na tym komputerze (menu Start, odinstalowanie w ustawieniach Windows).%nPrzenośna: same pliki, w wybranym folderze (np. na pendrivie); ustawienia i pobrane pliki zostają w tym folderze.
pl.ModeInstall=Instalacja (zalecana)
pl.ModePortable=Przenośna
nl.ModeTitle=Installatietype
nl.ModeSub=Hoe wil je Turtlefin gebruiken?
nl.ModeText=Installatie: Turtlefin wordt op deze computer geïnstalleerd (Startmenu, verwijderen via de Windows-instellingen).%nDraagbaar: alleen de bestanden, in een map naar keuze (bijvoorbeeld een USB-stick); instellingen en downloads blijven in die map.
nl.ModeInstall=Installatie (aanbevolen)
nl.ModePortable=Draagbaar

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked; Check: not IsPortable

[Files]
Source: "{#Src}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
; Marqueur du mode portable (voir paths.rs).
Source: "portable.txt"; DestDir: "{app}"; DestName: "portable"; Check: IsPortable; Flags: ignoreversion

[InstallDelete]
; Passage de portable à installé : le marqueur disparaît.
Type: files; Name: "{app}\portable"; Check: not IsPortable

[UninstallDelete]
Type: files; Name: "{app}\language"

[Icons]
Name: "{group}\Turtlefin"; Filename: "{app}\turtlefin.exe"; Check: not IsPortable
Name: "{group}\{cm:UninstallProgram,Turtlefin}"; Filename: "{uninstallexe}"; Check: not IsPortable
Name: "{autodesktop}\Turtlefin"; Filename: "{app}\turtlefin.exe"; Tasks: desktopicon

[Run]
; Lancement à la fin (aussi après une mise à jour silencieuse).
Filename: "{app}\turtlefin.exe"; Description: "{cm:LaunchProgram,Turtlefin}"; Flags: nowait postinstall

[Code]
var
  ModePage: TInputOptionWizardPage;

function IsPortable: Boolean;
begin
  if WizardSilent then
    // Mise à jour automatique : on garde le mode du dossier existant.
    Result := FileExists(ExpandConstant('{param:DIR|}') + '\portable')
  else
    Result := (ModePage <> nil) and (ModePage.SelectedValueIndex = 1);
end;

procedure InitializeWizard;
begin
  ModePage := CreateInputOptionPage(wpWelcome,
    CustomMessage('ModeTitle'), CustomMessage('ModeSub'), CustomMessage('ModeText'), True, False);
  ModePage.Add(CustomMessage('ModeInstall'));
  ModePage.Add(CustomMessage('ModePortable'));
  ModePage.SelectedValueIndex := 0;
end;

function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;
  // Portable : dossier proposé plus parlant que Program Files.
  if (CurPageID = ModePage.ID) and IsPortable and (Pos(ExpandConstant('{autopf}'), WizardForm.DirEdit.Text) = 1) then
    WizardForm.DirEdit.Text := ExpandConstant('{userdocs}\Turtlefin');
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  // Langue de l'assistant (en silencieux : celle de Windows, ou /LANG=) = langue de Turtlefin au premier
  // lancement. Sans effet ensuite : la langue choisie dans l'appli (prefs.json) prime.
  if CurStep = ssPostInstall then
    SaveStringToFile(ExpandConstant('{app}\language'), ActiveLanguage, False);
end;
