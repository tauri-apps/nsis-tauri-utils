#include <stdio.h>
#include <strsafe.h>
#include <tchar.h>
#include <windows.h>
#include <string>

#include "include\nsis\pluginapi.h"
#include "include\semver.hpp"

#define NSIS_MAX_STRLEN 1024

extern "C" void __declspec(dllexport) SemverCompare(HWND hwndParent,
                                                    int string_size,
                                                    TCHAR* variables,
                                                    stack_t** stacktop,
                                                    extra_parameters* extra) {
  EXDLL_INIT();

  TCHAR arg1[NSIS_MAX_STRLEN];
  if (popstringn(arg1, NSIS_MAX_STRLEN))
    return;

  TCHAR arg2[NSIS_MAX_STRLEN];
  if (popstringn(arg2, NSIS_MAX_STRLEN))
    return;

  std::wstring ver1w(arg1);
  std::wstring ver2w(arg2);

  std::string ver1(ver1w.begin(), ver1w.end());
  std::string ver2(ver2w.begin(), ver2w.end());

  semver::version v1 = semver::from_string(ver1);
  semver::version v2 = semver::from_string(ver2);

  pushint(v1.compare(v2));
}
