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
  ; Do not keep the container's working directory inside the runtime it must
  ; remove. Use the original EXE's directory while running the child GUI.
  SetOutPath "$EXEDIR"
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
  StrCpy $R0 $0
  ; WebView2 children can finish asynchronously after the GUI process exits.
  ; Resolve/check the exact owned extraction target before recursive cleanup;
  ; never apply this to data, EXEDIR or any environment-supplied storage path.
  GetFullPathName $R2 "$PLUGINSDIR"
  GetFullPathName $R3 "$PLUGINSDIR\app"
  ${If} $R2 != ""
  ${AndIf} $R3 == "$R2\app"
    StrCpy $R1 0
    cleanup_runtime:
      RMDir /r "$R3"
      IfFileExists "$R3" 0 cleanup_finished
      IntOp $R1 $R1 + 1
      ${If} $R1 < 40
        Sleep 250
        Goto cleanup_runtime
      ${EndIf}
    cleanup_finished:
  ${EndIf}
  SetErrorLevel $R0
  ; NSIS then cleans its unique plugin directory itself.
SectionEnd
