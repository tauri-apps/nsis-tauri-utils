---
"nsis_process": patch
"nsis_tauri_utils": patch
---

`KillProcess` and `KillProcessCurrentUser` will now be pushing 2 to the stack instead of 1 when no processes were found
