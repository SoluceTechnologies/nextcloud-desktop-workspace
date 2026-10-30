use super::*;
use std::fs;
use std::os::unix::fs::symlink;

fn add_card(drm: &Path, card: &str, driver: &str, boot_vga: &str) {
    let device = drm.join(card).join("device");
    fs::create_dir_all(&device).unwrap();
    fs::write(device.join("boot_vga"), format!("{boot_vga}\n")).unwrap();
    let driver_directory = drm.join("drivers").join(driver);
    fs::create_dir_all(&driver_directory).unwrap();
    symlink(driver_directory, device.join("driver")).unwrap();
}

#[test]
fn nvidia_drives_the_display_when_it_is_the_boot_gpu() {
    let drm = tempfile::tempdir().unwrap();
    add_card(drm.path(), "card1", "nvidia", "1");
    add_card(drm.path(), "card2", "amdgpu", "0");
    fs::create_dir_all(drm.path().join("card1-eDP-1")).unwrap();
    assert!(nvidia_drives_display(drm.path()));
}

#[test]
fn a_secondary_nvidia_gpu_does_not_drive_the_display() {
    let drm = tempfile::tempdir().unwrap();
    add_card(drm.path(), "card0", "amdgpu", "1");
    add_card(drm.path(), "card1", "nvidia", "0");
    assert!(!nvidia_drives_display(drm.path()));
}

#[test]
fn no_drm_devices_means_no_nvidia_display() {
    let drm = tempfile::tempdir().unwrap();
    assert!(!nvidia_drives_display(&drm.path().join("missing")));
}

#[test]
fn shared_memory_only_for_nvidia_displays_without_user_overrides() {
    let nothing_set = |_: &str| false;
    assert!(needs_shared_memory(true, nothing_set));
    assert!(!needs_shared_memory(false, nothing_set));
    for name in USER_OVERRIDES {
        assert!(!needs_shared_memory(true, |variable: &str| variable == name), "{name} should be respected");
    }
}
