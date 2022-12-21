Name "SemverCompare"
OutFile "SemverCompare.exe"
ShowInstDetails show

!addplugindir "Debug"
!addplugindir "Release"

Section "nsSemverCompare"
    SemverCompare::SemverCompare "1.2.1" "1.2.0"
    Pop $1
    DetailPrint $1
    SemverCompare::SemverCompare "1.2.0" "1.2.1"
    Pop $1
    DetailPrint $1
    SemverCompare::SemverCompare "1.2.1" "1.2.1"
    Pop $1
    DetailPrint $1
    SemverCompare::SemverCompare "1.2.1-alpha.1" "1.2.1-beta.5"
    Pop $1
    DetailPrint $1
    SemverCompare::SemverCompare "1.2.1-rc.1" "1.2.1-beta.1"
    Pop $1
    DetailPrint $1
    SemverCompare::SemverCompare "1.2.1-alpha.1" "1.2.1-alpha.1"
    Pop $1
    DetailPrint $1
    SemverCompare::SemverCompare "1.2qe2.1-alpha.1" "1.2.1-alpha.1"
    Pop $1
    DetailPrint $1
    SemverCompare::SemverCompare "1.2.1-alpha.1" "-q1.2.1-alpha.1"
    Pop $1
    DetailPrint $1
    SemverCompare::SemverCompare "1.2.saf1-alpha.1" "-q1.2.1-alpha.1"
    Pop $1
    DetailPrint $1
SectionEnd
