use super::*;

#[test]
fn list_local_dir_returns_empty_for_nonexistent_path() {
    // A path that cannot exist must not panic; the caller leaves any prior
    // listing intact by ignoring the empty result.
    let entries = list_local_dir("/this/path/does/not/exist/__meatshell_test__");
    assert!(entries.is_empty());
}

#[test]
fn list_local_dir_at_root_omits_parent_entry() {
    // The filesystem root has no parent, so the ".." entry is suppressed.
    let entries = list_local_dir("/");
    let has_parent = entries.iter().any(|e| e.name == "..");
    assert!(
        !has_parent,
        "root listing must not contain a parent (..) entry"
    );
}
