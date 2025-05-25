#![no_std]
use nsis_plugin_api::*;
nsis_plugin!();

/* start-marker */

/// Replaces all occurrences of a substring in a string with another substring.
///
/// Returns the modified string.
///
/// # Safety
///
/// This function always expects 3 strings on the stack ($string, $search, $replace) and will panic otherwise.
#[nsis_fn]
fn StrReplace() -> Result<(), Error> {
    let string = popstr()?;
    let search = popstr()?;
    let replace = popstr()?;

    let result = string.replace(&search, &replace);
    pushstr(&result)?;

    Ok(())
}

/* end-marker */
