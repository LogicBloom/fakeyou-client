pub mod error;

use std::time::Duration;

#[cfg(feature = "face_animator")]
use derive_builder::Builder;
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

    #[cfg(feature = "tts")]
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

    #[cfg(feature = "tts")]
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
                JobStatus::AttemptFailed | JobStatus::Pending | JobStatus::Started => {}
                JobStatus::CompleteSuccess => {
                    break Ok(response);
                }
                JobStatus::CompleteFailure | JobStatus::Dead => {
                    break Err(Error::TtsJobFailed(response.state.job_token));
                }
            }
            // sleep before making next request to prevent 429 errors
            std::thread::sleep(Duration::from_secs(8))
        }
    }

    pub fn request_file_url(&self, public_bucket_media_path: &str) -> String {
        format!("{FILE_STORAGE_BASE_URL}{public_bucket_media_path}")
    }

    #[cfg(feature = "voices")]
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

    #[cfg(feature = "face_animator")]
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

    #[cfg(feature = "face_animator")]
    pub async fn upload_image(&self, file: &[u8]) -> Result<UploadFileResponse, Error> {
        let payload = UploadFilePayload {
            uuid_idempotency_token: Uuid::new_v4(),
            file,
            source: "file",
        };
        let response = self
            .http_client
            .post(format!("{BASE_URL}/media_uploads/upload_image"))
            .form(&payload)
            .send()
            .await?
            .error_for_status()?
            .json::<UploadFileResponse>()
            .await?;
        Ok(response)
    }

    #[cfg(feature = "face_animator")]
    pub async fn create_facial_animation_builder(&self) -> CreateFaceAnimationPayloadBuilder {
        CreateFaceAnimationPayloadBuilder::create_empty()
    }

    #[cfg(feature = "face_animator")]
    pub async fn create_facial_animation(
        &self,
        payload: CreateFaceAnimationPayload,
    ) -> Result<CreateFaceAnimationResponse, Error> {
        let response = self
            .http_client
            .post(format!("{BASE_URL}/animation/face_animation/create"))
            .json(&payload)
            .send()
            .await?
            .error_for_status()?
            .json::<CreateFaceAnimationResponse>()
            .await?;
        Ok(response)
    }

    #[cfg(feature = "face_animator")]
    pub async fn poll_face_animation_job<T: Into<String> + Copy>(
        &self,
        inference_token: T,
    ) -> Result<FaceAnimationJobResponse, Error> {
        loop {
            let response = self
                .http_client
                .get(format!(
                    "{BASE_URL}/model_inference/job_status/{}",
                    inference_token.into()
                ))
                .send()
                .await?
                .error_for_status()?
                .json::<FaceAnimationJobResponse>()
                .await?;
            if !response.success {
                return Err(Error::FaceAnimationJobFailed(response));
            }
            match response.state.status.status {
                JobStatus::AttemptFailed | JobStatus::Pending | JobStatus::Started => {}
                JobStatus::CompleteSuccess => {
                    return Ok(response);
                }
                JobStatus::CompleteFailure | JobStatus::Dead => {
                    return Err(Error::FaceAnimationJobFailed(response));
                }
            }
            // sleep before making next request to prevent 429 errors
            std::thread::sleep(Duration::from_secs(10))
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TtsInferencePayload {
    uuid_idempotency_token: Uuid,
    tts_model_token: String,
    inference_text: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TtsInferenceResponse {
    pub success: bool,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
    pub error_reason: Option<String>,
    pub inference_job_token: Option<String>,
    pub inference_job_token_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TtsJobResponse {
    pub success: bool,
    pub state: TtsJobState,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TtsJobState {
    pub status: JobStatus,
    pub job_token: String,
    pub maybe_public_bucket_wav_audio_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all(deserialize = "snake_case"))]
pub enum JobStatus {
    AttemptFailed,
    CompleteFailure,
    CompleteSuccess,
    Dead,
    Pending,
    Started,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TtsVoice {
    pub model_token: String,
    pub tts_model_type: String,
    pub title: String,
    pub ietf_language_tag: String,
    pub ietf_primary_language_subtag: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct UploadFilePayload<'a> {
    uuid_idempotency_token: Uuid,
    file: &'a [u8],
    source: &'a str,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UploadFileResponse {
    pub success: bool,
    pub upload_token: String,
}

#[cfg(feature = "face_animator")]
#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateFaceAnimationPayload {
    #[builder(setter(custom))]
    audio_sorce: FaceAnimationMediaSource,
    #[builder(default = "\"twitter_square\".to_string()")]
    dimensions: String,
    disable_face_enhancement: bool,
    #[builder(setter(custom))]
    image_source: FaceAnimationMediaSource,
    make_still: bool,
    remove_watermark: bool,
    #[builder(default = "Uuid::new_v4()")]
    uuid_idempotency_token: Uuid,
}

#[cfg(feature = "face_animator")]
impl CreateFaceAnimationPayload {
    pub fn audio_sorce(&mut self, maybe_media_upload_token: String) {
        self.audio_sorce = FaceAnimationMediaSource {
            maybe_media_upload_token,
        };
    }

    pub fn image_source(&mut self, maybe_media_upload_token: String) {
        self.image_source = FaceAnimationMediaSource {
            maybe_media_upload_token,
        };
    }
}

#[cfg(feature = "face_animator")]
#[derive(Clone, Debug, Serialize)]
pub struct FaceAnimationMediaSource {
    maybe_media_upload_token: String,
}

#[cfg(feature = "face_animator")]
#[derive(Clone, Debug, Deserialize)]
pub struct CreateFaceAnimationResponse {
    pub success: bool,
    pub inference_job_token: String,
}

#[cfg(feature = "face_animator")]
#[derive(Clone, Debug, Deserialize)]
pub struct FaceAnimationJobResponse {
    pub success: bool,
    pub state: FaceAnimationJobState,
}

#[cfg(feature = "face_animator")]
#[derive(Clone, Debug, Deserialize)]
pub struct FaceAnimationJobState {
    pub job_token: String,
    pub request: FaceAnimationRequest,
    pub status: FaceAnimationStatus,
    pub maybe_result: Option<FaceAnimationRequest>,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(feature = "face_animator")]
#[derive(Clone, Debug, Deserialize)]
pub struct FaceAnimationRequest {
    pub inference_category: String,
    pub maybe_model_type: String,
    pub maybe_model_token: Option<String>,
    pub maybe_model_title: String,
    pub maybe_raw_inference_text: Option<String>,
}

#[cfg(feature = "face_animator")]
#[derive(Clone, Debug, Deserialize)]
pub struct FaceAnimationStatus {
    pub status: JobStatus,
    pub maybe_extra_status_description: Option<String>,
    pub maybe_assigned_worker: String,
    pub maybe_assigned_cluster: String,
    pub maybe_first_started_at: String,
    pub attempt_count: u32,
    pub require_keepalive: bool,
    pub maybe_failure_category: Option<String>,
}

#[cfg(feature = "face_animator")]
#[derive(Clone, Debug, Deserialize)]
pub struct FaceAnimationResult {
    pub entity_type: String,
    pub entity_token: String,
    pub maybe_public_bucket_media_path: String,
    pub maybe_successfully_completed_at: String,
}
