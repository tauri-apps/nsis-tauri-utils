#![cfg(all(windows, any(target_arch = "x86", target_arch = "x86_64")))]

//! Build and test release DLLs for the matching Cargo target and `ansi` feature:
//! `cargo test -p nsis-integration-tests --target i686-pc-windows-msvc -- --ignored --nocapture`
//! NSIS is found on PATH; `NSISDIR` overrides the installation.

#[cfg(all(feature = "ansi", not(target_arch = "x86")))]
compile_error!("The ANSI NSIS target is only supported on x86");

use std::{
    env, fs, io,
    os::windows::{io::AsRawHandle, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use windows_sys::Win32::{
    Foundation::WAIT_OBJECT_0,
    System::{
        SystemInformation::{IMAGE_FILE_MACHINE_AMD64, IMAGE_FILE_MACHINE_I386},
        Threading::{WaitForSingleObject, CREATE_NO_WINDOW},
    },
};

#[derive(Clone, Copy)]
struct NsisTarget {
    name: &'static str,
    machine: u16,
    rust_target: &'static str,
}

const PLUGINS: [&str; 4] = [
    "nsis-string",
    "nsis-semvercompare",
    "nsis-process",
    "nsis-tauri-utils",
];

const TARGETS: [NsisTarget; 3] = [
    NsisTarget {
        name: "x86-unicode",
        machine: IMAGE_FILE_MACHINE_I386,
        rust_target: "i686-pc-windows-msvc",
    },
    NsisTarget {
        name: "x86-ansi",
        machine: IMAGE_FILE_MACHINE_I386,
        rust_target: "i686-pc-windows-msvc",
    },
    NsisTarget {
        name: "amd64-unicode",
        machine: IMAGE_FILE_MACHINE_AMD64,
        rust_target: "x86_64-pc-windows-msvc",
    },
];

fn nsis_command() -> Command {
    let compiler = env::var_os("NSISDIR")
        .filter(|directory| !directory.is_empty())
        .map(|directory| Path::new(&directory).join("Bin/makensis.exe"))
        .unwrap_or_else(|| "makensis.exe".into());
    let mut command = Command::new(compiler);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

fn build_directory(target: NsisTarget) -> io::Result<PathBuf> {
    // Cargo places tests in <target-dir>/[<rust-target>/]<profile>/deps.
    let executable = env::current_exe()?;
    Ok(executable
        .ancestors()
        .nth(2)
        .unwrap()
        .join("nsis-tests")
        .join(target.name))
}

fn build_plugins(target: NsisTarget, build_directory: &Path) -> io::Result<PathBuf> {
    let mut cargo = Command::new(env!("CARGO"));
    cargo
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["build", "--release", "--target", target.rust_target])
        .arg("--target-dir")
        .arg(build_directory)
        .creation_flags(CREATE_NO_WINDOW);
    for package in PLUGINS {
        cargo.args(["--package", package]);
    }
    if cfg!(feature = "ansi") {
        cargo.args(["--features", "ansi"]);
    }

    println!("Building release DLLs for {}", target.name);
    let status = cargo.status()?;
    assert!(
        status.success(),
        "Plugin build failed for {}: {status}",
        target.name
    );
    Ok(build_directory.join(target.rust_target).join("release"))
}

struct ChildGuard(Child);

impl ChildGuard {
    fn wait(&mut self) -> io::Result<ExitStatus> {
        let status = unsafe { WaitForSingleObject(self.0.as_raw_handle().cast(), 30_000) };
        assert_eq!(
            status, WAIT_OBJECT_0,
            "child did not exit within 30 seconds"
        );
        self.0.wait()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn assert_pe_architecture(path: &Path, expected: u16) -> io::Result<()> {
    let bytes = fs::read(path)?;
    assert_eq!(&bytes[..2], b"MZ", "{} is not a PE file", path.display());
    let pe_offset = u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
    assert_eq!(&bytes[pe_offset..pe_offset + 4], b"PE\0\0");
    let machine = u16::from_le_bytes(bytes[pe_offset + 4..pe_offset + 6].try_into().unwrap());
    assert_eq!(machine, expected, "wrong architecture: {}", path.display());
    Ok(())
}

#[test]
#[ignore = "requires NSIS on PATH or in NSISDIR"]
fn native_installer() -> io::Result<()> {
    let target = if cfg!(target_arch = "x86_64") {
        TARGETS[2]
    } else if cfg!(feature = "ansi") {
        TARGETS[1]
    } else {
        TARGETS[0]
    };

    let build_directory = build_directory(target)?;

    let release_directory = build_plugins(target, &build_directory)?;
    run_target(target, &release_directory)
}

fn run_target(target: NsisTarget, release_directory: &Path) -> io::Result<()> {
    println!("Testing {}", target.name);

    // Verify that all plugin DLLs have the correct architecture for the target.
    for package in PLUGINS {
        let dll = release_directory.join(format!("{}.dll", package.replace('-', "_")));
        assert_pe_architecture(&dll, target.machine)?;
    }

    // Create a unique run directory for this test instance to avoid conflicts with other tests.
    let run_id = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let run_directory = release_directory.join("nsis-tests").join(&run_id);
    fs::create_dir_all(&run_directory)?;

    let fixture_prefix = format!("nsis-utils-{run_id}");

    // Run four uniquely named copies of ping so both plugins can test finding and
    // killing processes, including by current user. ChildGuard cleans up any survivors.
    let ping = Path::new(&env::var_os("WINDIR").expect("WINDIR must be set"))
        .join("System32")
        .join("ping.exe");
    let mut fixtures = Vec::new();
    for prefix in ["standalone", "combined"] {
        for operation in ["kill", "current"] {
            let path = run_directory.join(format!("{fixture_prefix}-{prefix}-{operation}.exe"));
            fs::copy(&ping, &path)?;
            fixtures.push(ChildGuard(
                Command::new(path)
                    .args(["-t", "127.0.0.1"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()?,
            ));
        }
    }

    // Compile the installer; NSIS reports an error if the target is unsupported.
    let installer = run_directory.join("plugin-tests.exe");
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins.nsi");
    let mut compiler = ChildGuard(
        nsis_command()
            .arg("/V2")
            .arg(format!("/DNSIS_TARGET={}", target.name))
            .arg(format!("/DPLUGIN_DIR={}", release_directory.display()))
            .arg(format!("/DOUTPUT_FILE={}", installer.display()))
            .arg(format!("/DFIXTURE_PREFIX={fixture_prefix}"))
            .arg(script)
            .spawn()
            .map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!(
                        "Could not run NSIS: {error}. Add makensis.exe to PATH or set NSISDIR."
                    ),
                )
            })?,
    );
    let status = compiler.wait()?;
    assert!(
        status.success(),
        "NSIS compilation failed for {}: {status}",
        target.name
    );
    assert_pe_architecture(&installer, target.machine)?;

    // Run the compiled installer and check the results.
    let mut process = ChildGuard(
        Command::new(&installer)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?,
    );
    let status = process.wait()?;
    let results = fs::read_to_string(run_directory.join("results.txt"))?;
    println!("{results}");
    assert!(status.success(), "installer failed: {status}");
    assert_eq!(results.lines().last(), Some("PASS"));
    println!(
        "Native {} installer checks passed: {}",
        target.name,
        installer.display()
    );
    Ok(())
}
