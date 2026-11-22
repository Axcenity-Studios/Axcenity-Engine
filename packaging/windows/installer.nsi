; Axcenity Editor installer. Built on GitHub Actions with:
;   makensis /DVERSION=... /DSTAGE=... /DOUTFILE=... /DICON=... /DHEADER=... /DSIDEBAR=... installer.nsi
;
; Current-user install (no admin). Pages: Welcome, License, Folder, Install,
; Finish (launch it, add a desktop shortcut). The pictures come from
; packaging/generate-assets.py, in the editor's dark theme.

Unicode True
SetCompressor /SOLID lzma
ManifestDPIAware true

!ifndef VERSION
  !error "Pass /DVERSION=0.1.0-alpha.1"
!endif
!ifndef STAGE
  !error "Pass /DSTAGE=path-to-staged-payload"
!endif
!ifndef OUTFILE
  !error "Pass /DOUTFILE=path-to-installer.exe"
!endif
!ifndef ICON
  !error "Pass /DICON=path-to-icon.ico"
!endif
!ifndef HEADER
  !error "Pass /DHEADER=path-to-header.bmp"
!endif
!ifndef SIDEBAR
  !error "Pass /DSIDEBAR=path-to-sidebar.bmp"
!endif

; 0.3.0-alpha.1 -> 0.3.0.0 for the Windows file version (the trailing "-"
; makes a version without a suffix parse too).
!searchparse "${VERSION}-" "" V_MAJOR "." V_MINOR "." V_PATCH "-"

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"

!define PRODUCT "Axcenity Editor"
!define EXE "axcenity-editor.exe"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\AxcenityEditor"
!define SETTINGS_KEY "Software\Axcenity\Editor"
; Where the editor keeps its settings and global assets: eframe's storage
; folder for APP_ID in crates/editor/src/main.rs.
!define SETTINGS_DIR "$APPDATA\Axcenity Editor"

Name "${PRODUCT}"
Caption "${PRODUCT} ${VERSION} Setup"
UninstallCaption "${PRODUCT} Uninstall"
BrandingText "Axcenity Engine ${VERSION}"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\Axcenity Engine"
InstallDirRegKey HKCU "${SETTINGS_KEY}" "InstallDir"
RequestExecutionLevel user
ShowInstDetails hide
ShowUninstDetails hide

!define MUI_ICON "${ICON}"
!define MUI_UNICON "${ICON}"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_RIGHT
!define MUI_HEADERIMAGE_BITMAP "${HEADER}"
!define MUI_HEADERIMAGE_UNBITMAP "${HEADER}"
; The pictures fill their place at any display scale (no white gaps at 125%
; or 150%); at 100% they're shown at their own size.
!define MUI_WELCOMEFINISHPAGE_BITMAP "${SIDEBAR}"
!define MUI_UNWELCOMEFINISHPAGE_BITMAP "${SIDEBAR}"
!define MUI_ABORTWARNING
!define MUI_ABORTWARNING_TEXT "Are you sure you want to stop installing ${PRODUCT}?"
!define MUI_UNABORTWARNING

!define MUI_WELCOMEPAGE_TITLE "Welcome to ${PRODUCT}"
!define MUI_WELCOMEPAGE_TEXT "Setup will install ${PRODUCT} ${VERSION} for your Windows account. You don't need administrator rights.$\r$\n$\r$\nBuild worlds from parts, script them in Luau or with blocks, press Play to test, and export your game for Windows, macOS and Linux.$\r$\n$\r$\nThis is an early release: expect rough edges, and keep backups of your projects.$\r$\n$\r$\nClick Next to continue."

!define MUI_LICENSEPAGE_TEXT_TOP "Axcenity Engine is free and open source under the Apache License 2.0."
!define MUI_LICENSEPAGE_TEXT_BOTTOM "Click I Agree to accept the license and continue."

!define MUI_DIRECTORYPAGE_TEXT_TOP "Setup will install ${PRODUCT} in the folder below. To use a different folder, click Browse.$\r$\n$\r$\nUpdates from inside the editor install into this same folder."

!define MUI_FINISHPAGE_TITLE "${PRODUCT} is ready"
!define MUI_FINISHPAGE_TEXT "${PRODUCT} is installed. You'll find it in the Start menu.$\r$\n$\r$\nNew versions are offered from inside the editor (Help > Check for Updates)."
!define MUI_FINISHPAGE_RUN "$INSTDIR\${EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Open ${PRODUCT}"
; The second checkbox adds a desktop shortcut (MUI's "show readme" slot).
!define MUI_FINISHPAGE_SHOWREADME ""
!define MUI_FINISHPAGE_SHOWREADME_TEXT "Add a shortcut to the desktop"
!define MUI_FINISHPAGE_SHOWREADME_FUNCTION DesktopShortcut
!define MUI_FINISHPAGE_LINK "Documentation and source on GitHub"
!define MUI_FINISHPAGE_LINK_LOCATION "https://github.com/MrBearson321/Axcenity-Engine"
!define MUI_FINISHPAGE_NOREBOOTSUPPORT

!define MUI_UNWELCOMEPAGE_TITLE "Remove ${PRODUCT}"
!define MUI_UNWELCOMEPAGE_TEXT "This will remove ${PRODUCT} ${VERSION} from your computer.$\r$\n$\r$\nYour project folders are never deleted. On the next page you can also remove your editor settings.$\r$\n$\r$\nClick Next to continue."
!define MUI_UNFINISHPAGE_NOREBOOTSUPPORT

VIProductVersion "${V_MAJOR}.${V_MINOR}.${V_PATCH}.0"
VIAddVersionKey "ProductName" "${PRODUCT}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "${PRODUCT} Setup"
VIAddVersionKey "LegalCopyright" "Copyright 2026 Paul Janota and contributors"
VIAddVersionKey "CompanyName" "Axcenity Engine"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${STAGE}\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_WELCOME
!define MUI_COMPONENTSPAGE_TEXT_TOP "Choose what to remove. Your project folders are never deleted."
!insertmacro MUI_UNPAGE_COMPONENTS
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "English"

; Asks to close the editor while it's running from the install folder, since
; its files can't be replaced then.
!macro WaitForEditorClosed UN
Function ${UN}WaitForEditorClosed
  ${If} ${FileExists} "$INSTDIR\${EXE}"
    ${Do}
      ClearErrors
      FileOpen $0 "$INSTDIR\${EXE}" a
      ${IfNot} ${Errors}
        FileClose $0
        ${Break}
      ${EndIf}
      MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "${PRODUCT} is open. Close it, then click Retry." /SD IDCANCEL IDRETRY +2
      Abort
    ${Loop}
  ${EndIf}
FunctionEnd
!macroend
!insertmacro WaitForEditorClosed ""
!insertmacro WaitForEditorClosed "un."

; Versions before 0.3 put a Start menu folder (chosen on its own page) with
; three shortcuts; now there's one shortcut. Removes the old folder.
!macro RemoveLegacyStartMenu
  ReadRegStr $1 HKCU "${SETTINGS_KEY}" "StartMenuFolder"
  ${If} $1 != ""
    Delete "$SMPROGRAMS\$1\Axcenity Editor.lnk"
    Delete "$SMPROGRAMS\$1\Install guide.lnk"
    Delete "$SMPROGRAMS\$1\Uninstall.lnk"
    RMDir "$SMPROGRAMS\$1"
    DeleteRegValue HKCU "${SETTINGS_KEY}" "StartMenuFolder"
  ${EndIf}
!macroend

Function DesktopShortcut
  CreateShortcut "$DESKTOP\${PRODUCT}.lnk" "$INSTDIR\${EXE}" "" "$INSTDIR\${EXE}" 0
FunctionEnd

Section "${PRODUCT}" SecEditor
  SectionIn RO
  Call WaitForEditorClosed
  SetOutPath "$INSTDIR"
  File /r "${STAGE}\*.*"
  File "${STAGE}\LICENSE"
  File "${STAGE}\NOTICE"
  File "${STAGE}\INSTALL.txt"

  WriteRegStr HKCU "${SETTINGS_KEY}" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "${PRODUCT}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "Paul Janota and contributors"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\${EXE}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "URLInfoAbout" "https://github.com/MrBearson321/Axcenity-Engine"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "HelpLink" "https://github.com/MrBearson321/Axcenity-Engine"
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "EstimatedSize" "$0"

  ; Collaborate invite links (axcenity://collaborate/join?...) open in the
  ; editor, which joins the session (C4 of docs/research/collaborate.md).
  WriteRegStr HKCU "Software\Classes\axcenity" "" "URL:Axcenity invite link"
  WriteRegStr HKCU "Software\Classes\axcenity" "URL Protocol" ""
  WriteRegStr HKCU "Software\Classes\axcenity\DefaultIcon" "" "$INSTDIR\${EXE},0"
  WriteRegStr HKCU "Software\Classes\axcenity\shell\open\command" "" '"$INSTDIR\${EXE}" "%1"'

  WriteUninstaller "$INSTDIR\Uninstall.exe"

  !insertmacro RemoveLegacyStartMenu
  CreateShortcut "$SMPROGRAMS\${PRODUCT}.lnk" "$INSTDIR\${EXE}" "" "$INSTDIR\${EXE}" 0
SectionEnd

Section "un.${PRODUCT}" UnSecEditor
  SectionIn RO
  Call un.WaitForEditorClosed
  Delete "$DESKTOP\${PRODUCT}.lnk"
  Delete "$SMPROGRAMS\${PRODUCT}.lnk"
  !insertmacro RemoveLegacyStartMenu

  RMDir /r "$INSTDIR"
  ; The version an in-app update replaced, kept for Roll back (crates/updater).
  RMDir /r "$INSTDIR.previous"

  DeleteRegKey HKCU "Software\Classes\axcenity"
  DeleteRegKey HKCU "${UNINSTALL_KEY}"
  DeleteRegKey HKCU "${SETTINGS_KEY}"
  DeleteRegKey /ifempty HKCU "Software\Axcenity"
SectionEnd

; Off unless ticked, so a silent uninstall (and an update) keeps them.
Section /o "un.Editor settings" UnSecSettings
  RMDir /r "${SETTINGS_DIR}"
SectionEnd

LangString DESC_UnSecEditor ${LANG_ENGLISH} "The editor, its players and its Start menu and desktop shortcuts."
LangString DESC_UnSecSettings ${LANG_ENGLISH} "Your preferences, recent projects, layouts and the global asset library in ${SETTINGS_DIR}. Leave this unticked to keep them for a later install."

!insertmacro MUI_UNFUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${UnSecEditor} $(DESC_UnSecEditor)
  !insertmacro MUI_DESCRIPTION_TEXT ${UnSecSettings} $(DESC_UnSecSettings)
!insertmacro MUI_UNFUNCTION_DESCRIPTION_END
