#![cfg_attr(target_os = "windows", no_std)]
use nsis_plugin::{nsis_fn, Nsis, Result};
// The Windows test harness supplies std's panic handler and allocator.
#[cfg(not(test))]
nsis_plugin::nsis_plugin!();

/* start-marker */
use core::cmp::Ordering;
use semver::Version;

nsis_fn! {
    /// Compare two semantic versions.
    ///
    /// Returns `0` if equal, `1` if `$v1` is newer and `-1` if `$v2` is newer.
    ///
    /// # Safety
    ///
    /// The stack must contain 2 strings ($v1, $v2).
    fn SemverCompare(nsis: &mut Nsis) -> Result<()> {
        let v1 = nsis.stack.pop()?;
        let v2 = nsis.stack.pop()?;

        match compare(&v1, &v2) {
            -1 => nsis.stack.push_int(-1)?,
            0 => nsis.stack.push_int(0)?,
            1 => nsis.stack.push_int(1)?,
            _ => unreachable!(),
        }

        Ok(())
    }
}

fn compare(v1: &str, v2: &str) -> i32 {
    let v1 = Version::parse(v1);
    let v2 = Version::parse(v2);

    let (v1, v2) = match (v1, v2) {
        (Ok(_), Err(_)) => return 1,
        (Err(_), Err(_)) => return 0,
        (Err(_), Ok(_)) => return -1,
        (Ok(v1), Ok(v2)) => (v1, v2),
    };

    match v1.cmp(&v2) {
        Ordering::Greater => 1,
        Ordering::Equal => 0,
        Ordering::Less => -1,
    }
}

/* end-marker */

#[cfg(test)]
mod tests {
    extern crate std;

    use std::string::ToString;

    use super::*;
    use nsis_plugin::testing::TestInstaller;

    #[test]
    fn it_works() {
        for (v1, v2, ret) in [
            ("1.2.1", "1.2.0", 1),
            ("1.2.0", "1.2.1", -1),
            ("1.2.1", "1.2.1", 0),
            ("1.2.1-alpha.1", "1.2.1-beta.5", -1),
            ("1.2.1-rc.1", "1.2.1-beta.1", 1),
            ("1.2.1-alpha.1", "1.2.1-alpha.1", 0),
            ("1.2qe2.1-alpha.1", "1.2.1-alpha.1", -1),
            ("1.2.1-alpha.1", "-q1.2.1-alpha.1", 1),
            ("1.2.saf1-alpha.1", "-q1.2.1-alpha.1", 0),
            ("1.0.0-aluc.0", "1.0.0", -1),
            (" 1.0.0-aluc.1", "1.0.0-bdfsf.0", -1),
            ("1.2.1-fffasd.1", "1.2.1-dasdqwe.1", 1),
            ("1.2.1-gasfdlkj.1", "1.2.1-calskjd.1", 1),
        ] {
            let mut installer = TestInstaller::stock();
            installer.push(v2);
            installer.push(v1);
            installer.call(SemverCompare);

            assert_eq!(installer.pop().as_deref(), Some(ret.to_string().as_str()));
            assert!(!installer.error());
        }
    }
}
