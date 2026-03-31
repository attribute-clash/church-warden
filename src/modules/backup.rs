use std::process::Command;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupTransport {
    Scp,
    Sftp,
}

pub struct BackupService;

impl BackupService {
    pub fn run_backup(
        db_file: &str,
        remote_target: &str,
        transport: BackupTransport,
    ) -> AppResult<()> {
        let status = match transport {
            BackupTransport::Scp => Command::new("scp").arg(db_file).arg(remote_target).status(),
            BackupTransport::Sftp => Command::new("bash")
                .arg("-lc")
                .arg(format!(
                    "printf 'put {}\n' | sftp {}",
                    db_file, remote_target
                ))
                .status(),
        }
        .map_err(|e| AppError::InvalidOperation(format!("backup command failed to start: {e}")))?;

        if !status.success() {
            return Err(AppError::InvalidOperation(format!(
                "backup command exited with status {status}"
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{BackupService, BackupTransport};

    #[test]
    fn invalid_backup_target_returns_error() {
        let result =
            BackupService::run_backup("/tmp/not-found.db", "bad@host:/tmp", BackupTransport::Scp);
        assert!(result.is_err());
    }
}
