#![cfg_attr(target_os = "windows", no_std)]

use nsis_plugin::{nsis_fn, Nsis, Result};

// The Windows test harness supplies std's panic handler and allocator.
#[cfg(not(test))]
nsis_plugin::nsis_plugin!();

include!(concat!(env!("OUT_DIR"), "/combined_libs.rs"));
