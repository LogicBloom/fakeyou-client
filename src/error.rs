use reqwest::StatusCode;

#[cfg(feature = "face_animator")]
use crate::FaceAnimationJobResponse;

#[derive(thiserror::Error)]
pub enum Error {
    #[error("Failed to authenticate user, check your credentials")]
    AuthenticationError,
    #[error("Too many requests")]
    TooManyRequestsError,
    #[error("Tts job '{0}' was unsuccessful")]
    TtsJobFailed(String),
    #[cfg(feature = "face_animator")]
    #[error("Face animation job was unsuccessful: {0:?}")]
    FaceAnimationJobFailed(FaceAnimationJobResponse),
    #[error(transparent)]
    InternalError(#[from] anyhow::Error),
}

impl From<reqwest::Error> for Error {
    #[allow(clippy::needless_return)]
    fn from(e: reqwest::Error) -> Self {
        if let Some(status) = e.status() {
            match status {
                StatusCode::UNAUTHORIZED => {
                    return Error::AuthenticationError;
                }
                StatusCode::TOO_MANY_REQUESTS => {
                    return Error::TooManyRequestsError;
                }
                _ => {
                    return Error::InternalError(e.into());
                }
            };
        } else {
            Error::InternalError(e.into())
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}
