; Anshot (安截) per-user installer. Built by scripts/package.ps1 with NSIS 3:
;   makensis /DVERSION=<x.y.z> /DSRC=<package dir> /DOUT=<setup.exe> installer/anshot.nsi
; Per-user install: no administrator rights, nothing written outside the user profile.
; Unsigned: Windows SmartScreen may warn on first run (see the release notes).

Unicode true
ManifestDPIAware true
RequestExecutionLevel user
SetCompressor /SOLID lzma

!ifndef VERSION
  !error "pass /DVERSION=x.y.z"
!endif
!ifndef SRC
  !error "pass /DSRC=<package dir>"
!endif
!ifndef OUT
  !define OUT "Anshot-${VERSION}-win-x64-setup.exe"
!endif

!define APP "Anshot"
!define APP_ZH "安截"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}"
!define RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"

Name "${APP_ZH} ${APP} ${VERSION}"
OutFile "${OUT}"
InstallDir "$LOCALAPPDATA\Programs\${APP}"
InstallDirRegKey HKCU "${UNINST_KEY}" "InstallLocation"
BrandingText "${APP_ZH} ${APP} ${VERSION}"

VIProductVersion "${VERSION}.0"
VIAddVersionKey /LANG=2052 "ProductName" "${APP_ZH} ${APP}"
VIAddVersionKey /LANG=2052 "FileDescription" "${APP_ZH} ${APP} 安装程序"
VIAddVersionKey /LANG=2052 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=2052 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=2052 "LegalCopyright" "MIT License"

!include "MUI2.nsh"
!include "LogicLib.nsh"
!define MUI_ICON "${SRC}\anshot.ico"
!define MUI_UNICON "${SRC}\anshot.ico"
!define MUI_ABORTWARNING
!define MUI_WELCOMEFINISHPAGE_BITMAP "${__FILEDIR__}\wizard.bmp"
!define MUI_UNWELCOMEFINISHPAGE_BITMAP "${__FILEDIR__}\wizard.bmp"
!define MUI_FINISHPAGE_RUN "$INSTDIR\Anshot.exe"
!define MUI_FINISHPAGE_RUN_TEXT "现在启动 ${APP_ZH}"
!define MUI_COMPONENTSPAGE_NODESC

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_COMPONENTS
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

; A running copy holds the exe open; it must be closed before files are replaced or removed.
; Interactive runs ask first (unsaved edits and pins would be lost); silent runs (/S) close it.
!macro CloseRunning
  System::Call 'kernel32::OpenMutexW(i 0x00100000, i 0, w "Local\Anshot.Desktop.Instance") p .r1'
  ${If} $1 P<> 0
    System::Call 'kernel32::CloseHandle(p r1)'
    ${IfNot} ${Silent}
    MessageBox MB_OKCANCEL|MB_ICONEXCLAMATION "安截正在运行。继续将关闭它，未复制或保存的编辑和钉图会丢失。$\n$\nAnshot is running. Continuing closes it; unsaved edits and pins are lost." IDOK +2
    Abort
    ${EndIf}
  ${EndIf}
  nsExec::Exec '"$SYSDIR\taskkill.exe" /IM Anshot.exe /F'
  Pop $0
  Sleep 400
!macroend

Section "${APP_ZH}（必需）" SecMain
  SectionIn RO
  !insertmacro CloseRunning
  SetOutPath "$INSTDIR"
  File "${SRC}\Anshot.exe"
  File "${SRC}\anshot.ico"
  File "${SRC}\README.txt"
  File "${SRC}\LICENSE.txt"
  File "${SRC}\THIRD_PARTY_LICENSES.md"
  File "${SRC}\FONT-OFL-1.1.txt"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateShortCut "$SMPROGRAMS\${APP_ZH} ${APP}.lnk" "$INSTDIR\Anshot.exe" "" "$INSTDIR\anshot.ico"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${APP_ZH} ${APP}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "Anshot contributors"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\anshot.ico"
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegStr HKCU "${UNINST_KEY}" "URLInfoAbout" "https://github.com/2023violet/Anshot"
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "EstimatedSize" 33000
SectionEnd

Section /o "桌面快捷方式" SecDesktop
  CreateShortCut "$DESKTOP\${APP_ZH} ${APP}.lnk" "$INSTDIR\Anshot.exe" "" "$INSTDIR\anshot.ico"
SectionEnd

Section "un.${APP_ZH}（必需）" UnSecMain
  SectionIn RO
  !insertmacro CloseRunning
  Delete "$INSTDIR\Anshot.exe"
  Delete "$INSTDIR\anshot.ico"
  Delete "$INSTDIR\README.txt"
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\THIRD_PARTY_LICENSES.md"
  Delete "$INSTDIR\FONT-OFL-1.1.txt"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${APP_ZH} ${APP}.lnk"
  Delete "$DESKTOP\${APP_ZH} ${APP}.lnk"
  ; Start at login is our own per-user value; never leave it pointing at a removed exe.
  DeleteRegValue HKCU "${RUN_KEY}" "${APP}"
  DeleteRegKey HKCU "${UNINST_KEY}"
SectionEnd

Section /o "un.同时删除设置与截图历史" UnSecData
  RMDir /r "$APPDATA\${APP}"
  RMDir /r "$LOCALAPPDATA\${APP}"
SectionEnd
