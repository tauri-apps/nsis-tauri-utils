# Changelog

## \[0.4.3]

- [`4054c7b`](https://www.github.com/tauri-apps/nsis-tauri-utils/commit/4054c7bd726288346099f6fa4044d8452c7baf9e) ([#49](https://www.github.com/tauri-apps/nsis-tauri-utils/pull/49) by [@Legend-Master](https://www.github.com/tauri-apps/nsis-tauri-utils/../../Legend-Master)) `KillProcess` and `KillProcessCurrentUser` will now be pushing 2 to the stack instead of 1 when no processes were found

## \[0.4.2]

- [`14d311b`](https://www.github.com/tauri-apps/nsis-tauri-utils/commit/14d311b13f598508cfa48fae66db3cfb5a30d3bf) ([#40](https://www.github.com/tauri-apps/nsis-tauri-utils/pull/40) by [@Legend-Master](https://www.github.com/tauri-apps/nsis-tauri-utils/../../Legend-Master)) Fix `RunAsUser` can't launch programs that require admin right
- [`c2c5e2f`](https://www.github.com/tauri-apps/nsis-tauri-utils/commit/c2c5e2f3733917635395d00881ef651255104e5d) ([#42](https://www.github.com/tauri-apps/nsis-tauri-utils/pull/42) by [@anatawa12](https://www.github.com/tauri-apps/nsis-tauri-utils/../../anatawa12)) Fix `KillProcessCurrentUser` and `KillProcess` may fails if the target processes are being terminated.

## \[0.4.1]

- [`3cb9d91`](https://www.github.com/tauri-apps/nsis-tauri-utils/commit/3cb9d9126a3e269ddfcf96617de08a73402182f2) ([#35](https://www.github.com/tauri-apps/nsis-tauri-utils/pull/35) by [@Legend-Master](https://www.github.com/tauri-apps/nsis-tauri-utils/../../Legend-Master)) Fix can't launch the app sometimes if the program path contains spaces

## \[0.4.0]

- [`8818f7c`](https://www.github.com/tauri-apps/nsis-tauri-utils/commit/8818f7cbfbf3f344f74508fccf9068c1eb58f52f) ([#32](https://www.github.com/tauri-apps/nsis-tauri-utils/pull/32)) Add `RunAsUser` to run command as unelevated user

## \[0.3.0]

- [`5423579`](https://www.github.com/tauri-apps/nsis-tauri-utils/commit/5423579860016c4f3074831eda03096ee4854e73)([#26](https://www.github.com/tauri-apps/nsis-tauri-utils/pull/26)) Reduce the DLL size by using `no_std` and without static msvcrt.

## \[0.2.2]

- [`7b6cfcc`](https://www.github.com/tauri-apps/nsis-tauri-utils/commit/7b6cfccd71c04a2ee87d6665b6822ccfe6d389b5)([#24](https://www.github.com/tauri-apps/nsis-tauri-utils/pull/24)) Add `FindProcessCurrentUser` and `KillProcessCurrentUser`.

## \[0.2.1]

- [`92f9264`](https://www.github.com/tauri-apps/nsis-tauri-utils/commit/92f92648b50fd298590570f43ed00de089609536)([#19](https://www.github.com/tauri-apps/nsis-tauri-utils/pull/19)) Skip processes with the same pid as the current installer's process to prevent the installer from killing itself.

## \[0.2.0]

- [`33ea4bc`](https://www.github.com/tauri-apps/nsis-tauri-utils/commit/33ea4bcf2a573461ebc5181ef2921d8746005049)([#17](https://www.github.com/tauri-apps/nsis-tauri-utils/pull/17)) Statically link CRT.

## \[0.1.0]

- Initial Release.
  - [000d632](https://www.github.com/tauri-apps/nsis-tauri-utils/commit/000d6326333f862741f1514de34542316445951e) ci: setup CI/CD and covector ([#2](https://www.github.com/tauri-apps/nsis-tauri-utils/pull/2)) on 2023-01-21
