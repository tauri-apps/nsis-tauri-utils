---
nsis_process: patch
nsis_tauri_utils: patch
---

Use an alternative method `CreateProcessWithTokenW` to run programs as user, this fixed a problem that the program launched with the previous method can't query its own handle
