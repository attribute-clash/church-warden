use church_warden::modules::backup::{BackupService, BackupTransport};

#[test]
fn backup_integration_bad_target_fails() {
    let result =
        BackupService::run_backup("church_warden.db", "invalid-target", BackupTransport::Scp);
    assert!(result.is_err());
}
