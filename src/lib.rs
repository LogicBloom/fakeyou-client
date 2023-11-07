pub mod error;

use std::time::Duration;

pub use error::Error;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

const BASE_URL: &str = "https://api.fakeyou.com";
const FILE_STORAGE_BASE_URL: &str = "https://storage.googleapis.com/vocodes-public";
const CARGO_PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct Client {
    http_client: HttpClient,
}

impl Client {
    pub async fn from_login_credentials<S: Into<String>>(
        username: S,
        password: S,
    ) -> Result<Self, Error> {
        let http_client = HttpClient::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .user_agent(format!(
                "chatterverse-fakeyou-client@{CARGO_PACKAGE_VERSION}"
            ))
            .cookie_store(true)
            .build()?;
        http_client
            .post(format!("{BASE_URL}/login"))
            .json(&json!({
                "username_or_email": username.into(),
                "password": password.into()
            }))
            .send()
            .await?
            .error_for_status()?;
        Ok(Client { http_client })
    }

    pub async fn tts_inference<S: Into<String>>(
        &self,
        tts_model_token: S,
        inference_text: S,
    ) -> Result<TtsInferenceResponse, Error> {
        let payload = TtsInferencePayload {
            uuid_idempotency_token: Uuid::new_v4(),
            tts_model_token: tts_model_token.into(),
            inference_text: inference_text.into(),
        };
        let response = self
            .http_client
            .post(format!("{BASE_URL}/tts/inference"))
            .json(&payload)
            .send()
            .await?
            .error_for_status()?
            .json::<TtsInferenceResponse>()
            .await?;
        Ok(response)
    }

    pub async fn poll_tts_job<S: Into<String> + Copy>(
        &self,
        inference_job_token: S,
    ) -> Result<TtsJobResponse, Error> {
        loop {
            let response = self
                .http_client
                .get(format!("{BASE_URL}/tts/job/{}", inference_job_token.into()))
                .send()
                .await?
                .error_for_status()?
                .json::<TtsJobResponse>()
                .await?;
            if !response.success {
                break Err(Error::TtsJobFailed(response.state.job_token));
            }
            match response.state.status {
                TtsJobStatus::Started | TtsJobStatus::Pending => {}
                TtsJobStatus::CompleteSuccess => {
                    break Ok(response);
                }
                TtsJobStatus::AttemptFailed
                | TtsJobStatus::CompleteFailure
                | TtsJobStatus::Dead => {
                    break Err(Error::TtsJobFailed(response.state.job_token));
                }
            }
            // sleep before making next request to prevent 429 errors
            std::thread::sleep(Duration::from_secs(10))
        }
    }

    pub fn request_audio_file(&self, wav_audio_path: &str) -> String {
        format!("{FILE_STORAGE_BASE_URL}{wav_audio_path}")
    }

    pub async fn voices(&self) -> Result<Vec<TtsVoice>, Error> {
        let response = self
            .http_client
            .get(format!("{BASE_URL}/tts/list"))
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?;
        let response = response.get("models").ok_or(anyhow::anyhow!(
            "Invalid response body: missing 'models' property"
        ))?;
        let response = serde_json::from_value(response.to_owned())
            .map_err(|_| anyhow::anyhow!("Failed to deserialize models"))?;
        Ok(response)
    }

    pub async fn upload_audio(&self, file: &[u8]) -> Result<UploadFileResponse, Error> {
        let payload = UploadFilePayload {
            uuid_idempotency_token: Uuid::new_v4(),
            file,
            source: "file",
        };
        let response = self
            .http_client
            .post(format!("{BASE_URL}/media_uploads/upload_audio"))
            .form(&payload)
            .send()
            .await?
            .error_for_status()?
            .json::<UploadFileResponse>()
            .await?;
        Ok(response)
    }
}

#[derive(Debug, Serialize)]
pub struct TtsInferencePayload {
    uuid_idempotency_token: Uuid,
    tts_model_token: String,
    inference_text: String,
}

#[derive(Debug, Deserialize)]
pub struct TtsInferenceResponse {
    pub success: bool,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
    pub error_reason: Option<String>,
    pub inference_job_token: Option<String>,
    pub inference_job_token_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TtsJobResponse {
    pub success: bool,
    pub state: TtsJobState,
}

#[derive(Debug, Deserialize)]
pub struct TtsJobState {
    pub status: TtsJobStatus,
    pub job_token: String,
    pub maybe_public_bucket_wav_audio_path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "snake_case"))]
pub enum TtsJobStatus {
    AttemptFailed,
    CompleteFailure,
    CompleteSuccess,
    Dead,
    Pending,
    Started,
}

#[derive(Debug, Deserialize)]
pub struct TtsVoice {
    pub model_token: String,
    pub tts_model_type: String,
    pub title: String,
    pub ietf_language_tag: String,
    pub ietf_primary_language_subtag: String,
}

#[derive(Debug, Serialize)]
pub struct UploadFilePayload<'a> {
    uuid_idempotency_token: Uuid,
    file: &'a [u8],
    source: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct UploadFileResponse {
    success: bool,
    upload_token: String,
}
