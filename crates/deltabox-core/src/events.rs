use anyhow::Result;
use rusqlite::params;
use uuid::Uuid;

use crate::util::now_rfc3339;
use crate::Vault;

impl Vault {
    pub(crate) fn record_event(
        &self,
        event_type: &str,
        subject_id: Option<&str>,
        subject_path: Option<&str>,
    ) -> Result<()> {
        let conn = self.open_db()?;
        conn.execute(
            "INSERT INTO events (event_id, event_type, file_id, subject_path, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                Uuid::new_v4().to_string(),
                event_type,
                subject_id,
                subject_path,
                now_rfc3339(),
            ],
        )?;
        Ok(())
    }
}
