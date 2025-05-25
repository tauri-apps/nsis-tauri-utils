#![no_std]
use nsis_plugin_api::*;
nsis_plugin!();

/* start-marker */
use core::cmp::Ordering;
use semver::Version;

/// Compare two semantic versions.
///
/// Returns `0` if equal, `1` if `$v1` is newer and `-1` if `$v2` is newer.
///
/// # Safety
///
/// This function always expects 2 strings on the stack ($v1, $v2) and will panic otherwise.
#[nsis_fn]
fn SemverCompare() -> Result<(), Error> {
    let v1 = popstr()?;
    let v2 = popstr()?;

    match compare(&v1, &v2) {
        -1 => push(NEGATIVE_ONE)?,
        0 => push(ZERO)?,
        1 => push(ONE)?,
        _ => unreachable!(),
    }

    Ok(())
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
    use super::*;

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
            assert_eq!(compare(v1, v2), ret);
        }
    }
}
