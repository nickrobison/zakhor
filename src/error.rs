use std::fmt;
use tokio::time::{Duration, sleep};

#[derive(Debug)]
pub enum ZakhorError {
    Database(String),
    NotFound(String),
    Validation(String),
    Internal(String),
}

impl fmt::Display for ZakhorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZakhorError::Database(msg) => write!(f, "Database error: {}", msg),
            ZakhorError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ZakhorError::Validation(msg) => write!(f, "Validation error: {}", msg),
            ZakhorError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for ZakhorError {}

impl From<ZakhorError> for String {
    fn from(err: ZakhorError) -> Self {
        err.to_string()
    }
}

pub type ZakhorResult<T> = anyhow::Result<T>;

#[allow(dead_code)]
pub async fn with_retry<F, Fut, T>(mut f: F, max_retries: u32) -> ZakhorResult<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = ZakhorResult<T>>,
{
    let mut last_error: Option<anyhow::Error> = None;

    for attempt in 0..=max_retries {
        let result = f().await;
        match result {
            Ok(result) => return Ok(result),
            Err(error) => {
                if let Some(ZakhorError::Database(_)) = error.downcast_ref::<ZakhorError>() {
                    last_error = Some(error);
                    if attempt < max_retries {
                        let delay = Duration::from_millis(100 * 2u64.pow(attempt));
                        sleep(delay).await;
                        continue;
                    }
                } else {
                    return Err(error);
                }
            }
        }
    }

    if let Some(err) = last_error {
        Err(err)
    } else {
        Err(ZakhorError::Internal("Retry failed unexpectedly".to_string()).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_display_database() {
        let err = ZakhorError::Database("connection lost".into());
        assert_eq!(err.to_string(), "Database error: connection lost");
    }

    #[test]
    fn test_display_not_found() {
        let err = ZakhorError::NotFound("item missing".into());
        assert_eq!(err.to_string(), "Not found: item missing");
    }

    #[test]
    fn test_display_validation() {
        let err = ZakhorError::Validation("bad input".into());
        assert_eq!(err.to_string(), "Validation error: bad input");
    }

    #[test]
    fn test_display_internal() {
        let err = ZakhorError::Internal("oops".into());
        assert_eq!(err.to_string(), "Internal error: oops");
    }

    #[test]
    fn test_error_trait() {
        let err = ZakhorError::Database("test".into());
        let err_ref: &dyn std::error::Error = &err;
        assert_eq!(err_ref.to_string(), "Database error: test");
    }

    #[test]
    fn test_into_string() {
        let err = ZakhorError::Validation("bad".into());
        let s: String = err.into();
        assert_eq!(s, "Validation error: bad");
    }

    #[tokio::test]
    async fn test_with_retry_success_first_try() {
        let result = with_retry(|| async { Ok::<_, anyhow::Error>(42) }, 3).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_retry_retries_then_succeeds() {
        let attempts = AtomicU32::new(0);
        let result = with_retry(
            || async {
                let n = attempts.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err(ZakhorError::Database("transient".into()).into())
                } else {
                    Ok("done")
                }
            },
            3,
        )
        .await;
        assert_eq!(result.unwrap(), "done");
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_with_retry_exhausts_retries() {
        let result = with_retry(
            || async {
                Err::<(), anyhow::Error>(ZakhorError::Database("persistent".into()).into())
            },
            2,
        )
        .await;
        let err = result.unwrap_err();
        assert!(matches!(
            err.downcast_ref::<ZakhorError>(),
            Some(ZakhorError::Database(_))
        ));
        assert_eq!(err.to_string(), "Database error: persistent");
    }

    #[tokio::test]
    async fn test_with_retry_propagates_non_database() {
        let result = with_retry(
            || async { Err::<(), anyhow::Error>(ZakhorError::NotFound("missing".into()).into()) },
            3,
        )
        .await;
        let err = result.unwrap_err();
        assert!(matches!(
            err.downcast_ref::<ZakhorError>(),
            Some(ZakhorError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn test_with_retry_zero_max_retries() {
        let result = with_retry(
            || async { Err::<(), anyhow::Error>(ZakhorError::Database("fail".into()).into()) },
            0,
        )
        .await;
        assert!(result.is_err());
    }
}
