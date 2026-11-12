# nsis-tauri-utils

A collection of NSIS plugins written in rust.

| Plugin                                             | Description                                                              |
| -------------------------------------------------- | ------------------------------------------------------------------------ |
| [nsis-process](./crates/nsis-process/)             | Find and Kill processes, and run program as unelevated user              |
| [nsis-semvercompare](./crates/nsis-semvercompare/) | Compare two semantic versions                                            |
| [nsis-string](./crates/nsis-string/)               | String manipulation                                                      |
| [nsis-tauri-utils](./crates/nsis-tauri-utils/)     | A collection of all the above plugins into a single DLL for smaller size |

## Release packages

Each plugin's ZIP contains its DLL under `amd64-unicode/`, `x86-unicode/`, and
`x86-ansi/`, with the same filename in each directory. Extract the archive into
your NSIS `Plugins` directory.

## License

Apache-2.0/MIT
