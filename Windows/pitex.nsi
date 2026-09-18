; Pitex per-user installer — double-click setup, no admin rights.
;
; Builds from Windows/ after build.sh has produced dist/pitex:
;   makensis -DVERSION=1.0.4 -DOUTFILE=Pitex-setup.exe pitex.nsi
;
; Installs to %LOCALAPPDATA%\Programs\Pitex (the standard per-user
; Programs location), adds Start Menu/Desktop shortcuts and an
; Add/Remove Programs entry. Silent mode (/S) is what the in-app
; updater drives: it closes a running copy, swaps the bundle, and
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

Name "Pitex"
OutFile "${OUTFILE}"
Unicode true
InstallDir "$LOCALAPPDATA\Programs\Pitex"
InstallDirRegKey HKCU "Software\Pitex" "InstallDir"
RequestExecutionLevel user

!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\bin\pitex.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch Pitex"

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"
  ; Silent installs are updater-driven — the caller waits for the app to
  ; exit, but kill a straggler anyway so the copy can't hit a locked file.
  ; Interactive installs leave a running copy alone (NSIS prompts on the
  ; locked file instead).
  ${If} ${Silent}
    nsExec::ExecToStack 'taskkill /F /IM pitex.exe'
    Pop $0
  ${EndIf}
  File /r "dist\pitex\*.*"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateDirectory "$SMPROGRAMS\Pitex"
  CreateShortcut "$SMPROGRAMS\Pitex\Pitex.lnk" "$INSTDIR\bin\pitex.exe"
  CreateShortcut "$SMPROGRAMS\Pitex\Uninstall Pitex.lnk" "$INSTDIR\uninstall.exe"
  CreateShortcut "$DESKTOP\Pitex.lnk" "$INSTDIR\bin\pitex.exe"
  WriteRegStr HKCU "Software\Pitex" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pitex" \
    "DisplayName" "Pitex"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pitex" \
    "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pitex" \
    "Publisher" "jaehwan-2ee"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pitex" \
    "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pitex" \
    "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pitex" \
    "DisplayIcon" "$INSTDIR\bin\pitex.exe"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pitex" \
    "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pitex" \
    "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pitex" \
    "EstimatedSize" $0
SectionEnd

Section "Uninstall"
  nsExec::ExecToStack 'taskkill /F /IM pitex.exe'
  Pop $0
  RMDir /r "$INSTDIR"
  RMDir /r "$SMPROGRAMS\Pitex"
  Delete "$DESKTOP\Pitex.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pitex"
  DeleteRegKey HKCU "Software\Pitex"
SectionEnd
