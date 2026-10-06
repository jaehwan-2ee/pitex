; Pitex per-user installer — double-click setup, no admin rights.
;
; Builds from Windows/ after build.sh has produced dist/<PITEX_BUNDLE>:
;   makensis -DVERSION=1.0.4 -DOUTFILE=Pitex-setup.exe \
;            -DNAME="Pitex Nightly" -DBINARY=pitex-nightly.exe \
;            -DZIPROOT=pitex-nightly -DREGKEY="Pitex Nightly" pitex.nsi
;
; Installs to %LOCALAPPDATA%\Programs\<NAME> (the standard per-user
; Programs location), adds Start Menu/Desktop shortcuts and an
; Add/Remove Programs entry. Silent mode (/S) is what the in-app
; updater drives: it waits for the running copy to exit, swaps the bundle, and
; the caller relaunches the new exe.

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef OUTFILE
  !define OUTFILE "Pitex-setup.exe"
!endif
!ifndef NAME
  !define NAME "Pitex"
!endif
!ifndef BINARY
  !define BINARY "pitex.exe"
!endif
!ifndef ZIPROOT
  !define ZIPROOT "pitex"
!endif
!ifndef REGKEY
  !define REGKEY "Pitex"
!endif

Name "${NAME}"
OutFile "${OUTFILE}"
Unicode true
InstallDir "$LOCALAPPDATA\Programs\${NAME}"
InstallDirRegKey HKCU "Software\${REGKEY}" "InstallDir"
RequestExecutionLevel user

!define MUI_ABORTWARNING
!define MUI_ICON "pitex.ico"
!define MUI_UNICON "pitex.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\bin\${BINARY}"
!define MUI_FINISHPAGE_RUN_TEXT "Launch ${NAME}"

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"
  ; The updater waits for its own process. Do not kill unrelated instances;
  ; a locked file must fail the install instead of relaunching a partial copy.
  ClearErrors
  File /r "dist\${ZIPROOT}\*.*"
  ${If} ${Errors}
    SetErrorLevel 1
    Quit
  ${EndIf}
  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateDirectory "$SMPROGRAMS\${NAME}"
  CreateShortcut "$SMPROGRAMS\${NAME}\${NAME}.lnk" "$INSTDIR\bin\${BINARY}"
  CreateShortcut "$SMPROGRAMS\${NAME}\Uninstall ${NAME}.lnk" "$INSTDIR\uninstall.exe"
  CreateShortcut "$DESKTOP\${NAME}.lnk" "$INSTDIR\bin\${BINARY}"
  WriteRegStr HKCU "Software\${REGKEY}" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${REGKEY}" \
    "DisplayName" "${NAME}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${REGKEY}" \
    "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${REGKEY}" \
    "Publisher" "jaehwan-2ee"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${REGKEY}" \
    "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${REGKEY}" \
    "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${REGKEY}" \
    "DisplayIcon" "$INSTDIR\bin\${BINARY}"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${REGKEY}" \
    "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${REGKEY}" \
    "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${REGKEY}" \
    "EstimatedSize" $0
SectionEnd

Section "Uninstall"
  nsExec::ExecToStack 'taskkill /F /IM ${BINARY}'
  Pop $0
  RMDir /r "$INSTDIR"
  RMDir /r "$SMPROGRAMS\${NAME}"
  Delete "$DESKTOP\${NAME}.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${REGKEY}"
  DeleteRegKey HKCU "Software\${REGKEY}"
SectionEnd
