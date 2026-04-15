use dmsx_core::DmsxError;

pub fn map_db_error(e: sqlx::Error) -> DmsxError {
    match &e {
        sqlx::Error::Database(dbe) => {
            let code = dbe.code().unwrap_or_default();
            match code.as_ref() {
                "23505" => DmsxError::Conflict("resource already exists".into()),
                "23503" => DmsxError::Validation("referenced resource does not exist".into()),
                "23514" => DmsxError::Validation("check constraint violated".into()),
                _ => {
                    tracing::error!(pg_code = %code, "unhandled database error: {e}");
                    DmsxError::Internal("database error".into())
                }
            }
        }
        _ => {
            tracing::error!("database error: {e}");
            DmsxError::Internal("database error".into())
        }
    }
}
