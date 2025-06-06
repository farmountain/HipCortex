use hipcortex::snapshot_manager::SnapshotManager;
use std::fs::{self, File};

#[test]
fn save_and_load() {
    let file = "snapshot_source.txt";
    fs::write(file, "hi").unwrap();
    let archive = SnapshotManager::save(file, "test").unwrap();
    let dest_dir = "snapshot_dest";
    let _ = fs::create_dir(dest_dir);
    SnapshotManager::load(&archive, dest_dir).unwrap();
    assert!(fs::metadata(format!("{}/{}", dest_dir, file)).is_ok());
    fs::remove_file(file).unwrap();
    fs::remove_file(archive).unwrap();
    fs::remove_dir_all(dest_dir).unwrap();
}
