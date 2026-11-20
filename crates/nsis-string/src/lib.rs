#![cfg_attr(target_os = "windows", no_std)]
use nsis_plugin::{nsis_fn, Nsis, Result};
// The Windows test harness supplies std's panic handler and allocator.
#[cfg(not(test))]
nsis_plugin::nsis_plugin!();

/* start-marker */

nsis_fn! {
    /// Replaces all occurrences of a substring in a string with another substring.
    ///
    /// Returns the modified string.
    ///
    /// # Safety
    ///
    /// The stack must contain 3 strings ($string, $search, $replace).
    fn StrReplace(nsis: &mut Nsis) -> Result<()> {
        let string = nsis.stack.pop()?;
        let search = nsis.stack.pop()?;
        let replace = nsis.stack.pop()?;

        let result = string.replace(&search, &replace);
        nsis.stack.push(&result)?;

        Ok(())
    }
}

/* end-marker */
