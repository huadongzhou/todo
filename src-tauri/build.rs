fn main() {
    declare_common_controls_for_test_binaries();
    tauri_build::build()
}

/// Make the Common-Controls 6.0.0.0 side-by-side assembly available to the test executables on
/// Windows MSVC targets.
///
/// The dependency chain imports `comctl32.dll` entry points (`TaskDialogIndirect`,
/// `SetWindowSubclass`, `RemoveWindowSubclass`, `DefSubclassProc`) that only exist in the
/// Common-Controls 6.0.0.0 assembly. `tauri_build::build()` embeds an application manifest
/// through a resource that Cargo links into bin targets only, so the `--lib` test harness
/// executable binds to the v5.82 `System32\comctl32.dll` and fails to start with
/// STATUS_ENTRYPOINT_NOT_FOUND (0xC0000139).
///
/// Cargo cannot scope linker arguments to the lib test target (`rustc-link-arg-tests` only covers
/// `tests/` integration targets), so the dependency is declared for every linked target and
/// written to the linker generated side-by-side manifest file, which the loader reads for
/// executables that carry no embedded manifest. Embedding it instead would collide with the
/// manifest resource Tauri already links into bin targets (CVT1100 duplicate resource), hence the
/// explicit `/MANIFEST:NO` for bins: they keep the embedded Tauri manifest and produce no extra
/// file, so packaging stays unchanged.
fn declare_common_controls_for_test_binaries() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os != "windows" || target_env != "msvc" {
        return;
    }

    println!("cargo:rustc-link-arg=/MANIFEST");
    println!(
        "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' \
         name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' \
         publicKeyToken='6595b64144ccf1df' language='*'"
    );
    println!("cargo:rustc-link-arg-bins=/MANIFEST:NO");
}
