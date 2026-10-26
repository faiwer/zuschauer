//! Every sysctl in the kernel's temperature format (`IK`) under `dev` and
//! `hw.acpi`, found by walking the tree like `sysctl(8)` does. Drivers name
//! them differently, `dev.cpu.0.temperature` (coretemp) or
//! `dev.amdtemp.0.core0.sensor0`, so the format is what marks a sensor. The
//! sysctl name is the sensor's ID.

use std::ffi::{CStr, CString};
use std::io;

pub(super) const ROOT: &str = "the dev and hw.acpi sysctls";
const TREES: [&str; 2] = ["dev", "hw.acpi"];
/// `CTL_MAXNAME`.
const MAX_DEPTH: usize = 24;
const INT: usize = size_of::<libc::c_int>();

/// The magic `{0, n, …}` OIDs that walk the tree, see `sysctl(3)`.
const NAME: libc::c_int = 1;
const NEXT: libc::c_int = 2;
const NAME_TO_OID: libc::c_int = 3;
const FORMAT: libc::c_int = 4;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Sysctl {
    name: CString,
    /// Kelvin per unit: `IK` is tenths, `IK3` thousandths.
    kelvin_per_unit: f64,
}

#[derive(Debug)]
pub(super) struct Found {
    pub id: String,
    pub path: Sysctl,
}

/// Sorted by ID.
pub(super) fn discover() -> io::Result<Vec<Found>> {
    let mut found = Vec::new();
    for tree in TREES {
        let Some(root) = name_to_oid(tree)? else {
            continue;
        };
        let mut oid = root.clone();
        while let Some(next) = next_oid(&oid)? {
            if !next.starts_with(&root) {
                break;
            }
            if let Some(kelvin_per_unit) = format(&next)?.as_deref().and_then(temperature_scale) {
                let id = oid_to_name(&next)?;
                found.push(Found {
                    path: Sysctl {
                        name: CString::new(id.clone()).map_err(io::Error::other)?,
                        kelvin_per_unit,
                    },
                    id,
                });
            }
            oid = next;
        }
    }
    found.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(found)
}

/// Degrees Celsius.
pub(super) fn read(sysctl: &Sysctl) -> io::Result<f32> {
    let mut value = [0u8; INT];
    let mut size = value.len();
    // SAFETY: `name` is NUL-terminated and `value` is writable for `size`.
    let status = unsafe {
        libc::sysctlbyname(
            sysctl.name.as_ptr(),
            value.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null(),
            0,
        )
    };
    if status != 0 {
        return Err(io::Error::last_os_error());
    }
    let value = libc::c_int::from_ne_bytes(value);
    Ok(celsius(value, sysctl.kelvin_per_unit))
}

fn celsius(value: libc::c_int, kelvin_per_unit: f64) -> f32 {
    (f64::from(value) * kelvin_per_unit - 273.15) as f32
}

/// `IK` and `IKn`: an integer in tenths of a kelvin, or in 10^-n.
fn temperature_scale(format: &str) -> Option<f64> {
    let precision: i32 = match format.strip_prefix("IK")? {
        "" => 1,
        digits => digits.parse().ok().filter(|p| (0..=9).contains(p))?,
    };
    Some(10f64.powi(-precision))
}

fn name_to_oid(name: &str) -> io::Result<Option<Vec<libc::c_int>>> {
    let mut oid = [0u8; MAX_DEPTH * INT];
    let size = sysctl(&[0, NAME_TO_OID], &mut oid, name.as_bytes());
    or_none(size.map(|size| ints(&oid[..size])))
}

/// The next leaf after `oid`, depth first.
fn next_oid(oid: &[libc::c_int]) -> io::Result<Option<Vec<libc::c_int>>> {
    let mut next = [0u8; MAX_DEPTH * INT];
    let size = sysctl(&[&[0, NEXT], oid].concat(), &mut next, &[]);
    or_none(size.map(|size| ints(&next[..size])))
}

fn oid_to_name(oid: &[libc::c_int]) -> io::Result<String> {
    let mut name = [0u8; 1024];
    let size = sysctl(&[&[0, NAME], oid].concat(), &mut name, &[])?;
    Ok(c_string(&name[..size]))
}

/// The format string, like `IK` or `A`, after a `u_int` kind. `None` for
/// nodes without one.
fn format(oid: &[libc::c_int]) -> io::Result<Option<String>> {
    let mut buffer = [0u8; 256];
    let size = sysctl(&[&[0, FORMAT], oid].concat(), &mut buffer, &[]);
    let format = size.map(|size| buffer.get(INT..size).map(c_string));
    Ok(or_none(format)?.flatten())
}

fn or_none<T>(result: io::Result<T>) -> io::Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(e) if e.raw_os_error() == Some(libc::ENOENT) => Ok(None),
        Err(e) => Err(e),
    }
}

fn ints(bytes: &[u8]) -> Vec<libc::c_int> {
    bytes
        .as_chunks::<INT>()
        .0
        .iter()
        .map(|chunk| libc::c_int::from_ne_bytes(*chunk))
        .collect()
}

fn c_string(bytes: &[u8]) -> String {
    CStr::from_bytes_until_nul(bytes)
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|_| String::from_utf8_lossy(bytes).into_owned())
}

/// Reads into `old`, writing `new` unless it's empty. Returns the number of
/// bytes read.
fn sysctl(mib: &[libc::c_int], old: &mut [u8], new: &[u8]) -> io::Result<usize> {
    let mut size = old.len();
    let new_ptr = if new.is_empty() {
        std::ptr::null()
    } else {
        new.as_ptr().cast()
    };
    // SAFETY: `old` is writable for `size` bytes and `new` is only read.
    let status = unsafe {
        libc::sysctl(
            mib.as_ptr(),
            mib.len() as libc::c_uint,
            old.as_mut_ptr().cast(),
            &mut size,
            new_ptr,
            new.len(),
        )
    };
    if status != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_kelvin_formats() {
        let tenths = temperature_scale("IK").unwrap();
        assert!((celsius(3232, tenths) - 50.05).abs() < 1e-3);
        let thousandths = temperature_scale("IK3").unwrap();
        assert!(celsius(273_150, thousandths).abs() < 1e-3);
        assert_eq!(temperature_scale("I"), None);
        assert_eq!(temperature_scale("IKx"), None);
        assert_eq!(temperature_scale("A"), None);
    }

    /// The walk itself, on a tree every FreeBSD has.
    #[test]
    fn walks_the_tree() {
        let root = name_to_oid("kern").unwrap().unwrap();
        let mut oid = root.clone();
        let mut ostype = None;
        while let Some(next) = next_oid(&oid).unwrap() {
            if !next.starts_with(&root) {
                break;
            }
            if oid_to_name(&next).unwrap() == "kern.ostype" {
                ostype = format(&next).unwrap();
            }
            oid = next;
        }
        assert_eq!(ostype.as_deref(), Some("A"));
        assert_eq!(name_to_oid("no.such.tree").unwrap(), None);
    }
}
