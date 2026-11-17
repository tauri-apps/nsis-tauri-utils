; Run by the Rust installer integration test with matching DLL and installer architectures.
Target ${NSIS_TARGET}

!if "${NSIS_TARGET}" == "x86-ansi"
    Unicode false
!else
    Unicode true
!endif

Name "NSIS plugin integration tests"
OutFile "${OUTPUT_FILE}"
RequestExecutionLevel user
SilentInstall silent
!addplugindir "${PLUGIN_DIR}"

Var Report

!macro Check Expected Description
    Pop $R0
    StrCmpS $R0 "${Expected}" +4
    FileWrite $Report "FAIL ${Description}: expected ${Expected}, got $R0$\r$\n"
    SetErrorLevel 1
    Quit
    FileWrite $Report "PASS ${Description}$\r$\n"
!macroend

!macro ExercisePlugins StringPlugin SemverPlugin ProcessPlugin Prefix
    Push "stack sentinel"
    ${SemverPlugin}::SemverCompare "1.0.0" "1.1.0"
    !insertmacro Check "-1" "${Prefix}: older version"
    ${SemverPlugin}::SemverCompare "2.0.0" "1.1.0"
    !insertmacro Check "1" "${Prefix}: newer version"
    ${SemverPlugin}::SemverCompare "1.0.0" "1.0.0"
    !insertmacro Check "0" "${Prefix}: equal versions"

    ${StringPlugin}::StrReplace "hello hello" "hello" "world"
    !insertmacro Check "world world" "${Prefix}: string replacement"

    !if "${NSIS_TARGET}" == "x86-ansi"
        ${StringPlugin}::StrReplace "café" "z" "a"
        !insertmacro Check "café" "${Prefix}: ANSI code-page round trip"
    !else
        ${StringPlugin}::StrReplace "مرحبا 🌍 مرحبا" "مرحبا" "你好"
        !insertmacro Check "你好 🌍 你好" "${Prefix}: UTF-16 and surrogate pairs"
    !endif

    ${StringPlugin}::StrReplace "" "x" "y"
    !insertmacro Check "" "${Prefix}: empty string"

    ; Build a string that fills the host's buffer, then expand it beyond that limit.
    StrCpy $1 "x"

    ${Prefix}_fill_string:
    StrCpy $2 $1
    StrCpy $1 "$1$1"
    StrCmp $1 $2 0 ${Prefix}_fill_string
    ${StringPlugin}::StrReplace "$1" "x" "xx"
    !insertmacro Check "$1" "${Prefix}: output truncated to host string capacity"

    ${ProcessPlugin}::FindProcess "${FIXTURE_PREFIX}-${Prefix}-kill.exe"
    !insertmacro Check "0" "${Prefix}: find running fixture"
    ${ProcessPlugin}::FindProcessCurrentUser "${FIXTURE_PREFIX}-${Prefix}-current.exe"
    !insertmacro Check "0" "${Prefix}: find current user's fixture"
    ${ProcessPlugin}::KillProcess "${FIXTURE_PREFIX}-${Prefix}-kill.exe"
    !insertmacro Check "0" "${Prefix}: kill fixture"
    ${ProcessPlugin}::KillProcessCurrentUser "${FIXTURE_PREFIX}-${Prefix}-current.exe"
    !insertmacro Check "0" "${Prefix}: kill current user's fixture"
    Sleep 100
    ${ProcessPlugin}::FindProcess "${FIXTURE_PREFIX}-${Prefix}-kill.exe"
    !insertmacro Check "1" "${Prefix}: killed fixture is absent"
    ${ProcessPlugin}::FindProcess "${FIXTURE_PREFIX}-${Prefix}-current.exe"
    !insertmacro Check "1" "${Prefix}: current user's killed fixture is absent"

    ${ProcessPlugin}::RunAsUser "$SYSDIR\cmd.exe" '/d /c echo passed>"$EXEDIR\run-${Prefix}.txt"'
    !insertmacro Check "0" "${Prefix}: launch process"
    StrCpy $2 0

    ${Prefix}_wait_for_child:
    IfFileExists "$EXEDIR\run-${Prefix}.txt" ${Prefix}_child_finished
    Sleep 100
    IntOp $2 $2 + 1
    IntCmp $2 100 0 ${Prefix}_wait_for_child
    FileWrite $Report "FAIL ${Prefix}: launched process did not write its marker$\r$\n"
    SetErrorLevel 1
    Quit

    ${Prefix}_child_finished:
    !insertmacro Check "stack sentinel" "${Prefix}: stack remains balanced"
!macroend

Section
    FileOpen $Report "$EXEDIR\results.txt" w
    !insertmacro ExercisePlugins nsis_string nsis_semvercompare nsis_process standalone
    !insertmacro ExercisePlugins nsis_tauri_utils nsis_tauri_utils nsis_tauri_utils combined
    FileWrite $Report "PASS$\r$\n"
    FileClose $Report
    SetErrorLevel 0
SectionEnd
