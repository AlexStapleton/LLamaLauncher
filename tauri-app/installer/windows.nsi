; Portable executable container. Only NSIS-owned temporary files are extracted.
Unicode true
!include "LogicLib.nsh"
!include "x64.nsh"
Name "LLamaLauncher"
OutFile "${OUTPUT}"
Icon "${ICON}"
RequestExecutionLevel user
SilentInstall silent
AutoCloseWindow true
SetCompressor zlib
CRCCheck force
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "LLamaLauncher"
VIAddVersionKey "FileDescription" "LLamaLauncher Portable"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "LLamaLauncher contributors"

Function .onInit
  SetSilent silent
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "LLamaLauncher requires 64-bit Windows."
    SetErrorLevel 2
    Quit
  ${EndIf}
FunctionEnd

Section "Run"
  InitPluginsDir
  SetOutPath "$PLUGINSDIR\app"
  ClearErrors
  File /r "${STAGE}\*"
  ${If} ${Errors}
    MessageBox MB_ICONSTOP "Could not unpack LLamaLauncher's runtime. Check free space in your temporary folder."
    SetErrorLevel 3
    Quit
  ${EndIf}
  ; Storage resolves from the downloaded EXE, rather than its temporary copy.
  ; No registration, shortcuts, system runtime or persistent app extraction.
  System::Call 'kernel32::SetEnvironmentVariableW(w "LLAMA_LAUNCHER_PORTABLE_EXE", w "$EXEPATH") i .r0'
  ${If} $0 == 0
    SetErrorLevel 4
    Quit
  ${EndIf}
  run_gui:
    ClearErrors
    ExecWait '"$PLUGINSDIR\app\llama-server-configurator.exe"' $0
    ${If} ${Errors}
      MessageBox MB_ICONSTOP "Could not start LLamaLauncher."
      SetErrorLevel 5
      Quit
    ${EndIf}
    ; Saved-data recovery asks the container to restart the GUI, keeping the
    ; runtime alive until the replacement GUI has also exited.
    ${If} $0 == 75
      Goto run_gui
    ${EndIf}
  SetErrorLevel $0
  ; NSIS cleans only its unique $PLUGINSDIR after the GUI exits.
SectionEnd
