// orz — assurance-first CLI agent workbench.
// Phase 1 scaffold: minimal binary that verifies the build system.
// orz-shell and orz-tui will be linked in Phase 2-3.

fn main() {
    println!(
        "orz {} — assurance-first CLI agent workbench",
        env!("CARGO_PKG_VERSION")
    );
    println!("Phase 1 scaffold: build system verified.");
}
