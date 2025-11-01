---
"nsis_process": patch
"nsis_tauri_utils": patch
---

`KillProcess` and `KillProcessCurrentUser` should be pushing 0 to the stack instead of 1 when no processes were found
