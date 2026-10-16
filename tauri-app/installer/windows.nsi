Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"
Name "LLamaLauncher"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\LLamaLauncher"
InstallDirRegKey HKCU "Software\LLamaLauncher" "InstallDir"
RequestExecutionLevel user
; WebView2 is large. Standard zlib keeps setup builds/installs fast and permits
; independent extraction of files; no outer archive is required by the user.
SetCompressor zlib
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "LLamaLauncher"
VIAddVersionKey "FileDescription" "LLamaLauncher Setup"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "LLamaLauncher contributors"
!define MUI_ICON "${ICON}"
!define MUI_UNICON "${ICON}"
!define MUI_WELCOMEPAGE_TEXT "Install LLamaLauncher for your Windows account.$\r$\n$\r$\nIncludes the GUI runtime. llama.cpp and models are downloaded separately inside the app.$\r$\n$\r$\nProfiles, engines and models remain in data beside the application. Choose a writable folder. Updates and uninstall preserve data."
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "LLamaLauncher requires 64-bit Windows."
    Abort
  ${EndIf}
  SetShellVarContext current
FunctionEnd

!macro CheckClosed
  FindWindow $0 "" "Llama Server Configuration Manager"
  ${If} $0 != 0
    IfSilent +2
    MessageBox MB_ICONSTOP "Close LLamaLauncher and stop its server before continuing."
    SetErrorLevel 2
    Quit
  ${EndIf}
!macroend

Section "LLamaLauncher"
  !insertmacro CheckClosed
  SetOutPath "$INSTDIR"
  File /r "${STAGE}\*"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\LLamaLauncher"
  CreateShortcut "$SMPROGRAMS\LLamaLauncher\LLamaLauncher.lnk" "$INSTDIR\llama-server-configurator.exe"
  CreateShortcut "$DESKTOP\LLamaLauncher.lnk" "$INSTDIR\llama-server-configurator.exe"
  WriteRegStr HKCU "Software\LLamaLauncher" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LLamaLauncher" "DisplayName" "LLamaLauncher"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LLamaLauncher" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LLamaLauncher" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LLamaLauncher" "DisplayIcon" "$INSTDIR\llama-server-configurator.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LLamaLauncher" "UninstallString" '$"$INSTDIR\Uninstall.exe$"'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LLamaLauncher" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LLamaLauncher" "NoRepair" 1
SectionEnd

Section "Uninstall"
  !insertmacro CheckClosed
  SetShellVarContext current
  ; Exact packaged files only. No recursive directory deletion, and never data.
  !include "${OWNED_FILES}"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\LLamaLauncher\LLamaLauncher.lnk"
  RMDir "$SMPROGRAMS\LLamaLauncher"
  Delete "$DESKTOP\LLamaLauncher.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LLamaLauncher"
  DeleteRegKey HKCU "Software\LLamaLauncher"
SectionEnd
