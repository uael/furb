//! The Responses API of a `ChatGPT` subscription, on the sign-in that furb keeps.

use super::catalog;
use std::path::{Path, PathBuf};

use rig_core::{
  client::CompletionClient,
  completion::{CompletionError, CompletionModel, CompletionRequest, CompletionResponse},
  http_client::{self, HeaderMap, HeaderValue, StreamingResponse},
  providers::openai,
  streaming::StreamingCompletionResponse,
};
use serde_json::{Value, json};

use super::clients::{Amend, Amended};

mod auth;

pub use auth::{AuthError, NAME, SignIn, begin, cancel, refresh, status, subscribe};

#[derive(Clone)]
pub(super) struct Codex {
  pub config: PathBuf,
  pub id: String,
}

impl CompletionModel for Codex {
  async fn completion(
    &self,
    request: CompletionRequest,
  ) -> Result<CompletionResponse, CompletionError> {
    use futures::StreamExt;
    let mut stream = self.stream(request).await?;
    while let Some(part) = stream.next().await {
      part?;
    }
    let raw = stream.response.as_ref().map(|end| end.raw.clone()).unwrap_or_default();
    Ok(CompletionResponse::from(stream).with_raw(raw))
  }

  async fn stream(
    &self,
    mut request: CompletionRequest,
  ) -> Result<StreamingCompletionResponse, CompletionError> {
    let credential = auth::credential(&self.config).await.map_err(|no| {
      auth::failed(&self.config, &no);
      CompletionError::ProviderError(no.to_string())
    })?;
    let mut headers = HeaderMap::new();
    headers.insert(
      "chatgpt-account-id",
      HeaderValue::from_str(&credential.account)
        .map_err(|no| CompletionError::ProviderError(no.to_string()))?,
    );
    headers.insert("originator", HeaderValue::from_static("furb"));
    headers.insert("openai-beta", HeaderValue::from_static("responses=experimental"));
    headers.insert("accept", HeaderValue::from_static("text/event-stream"));
    headers.insert("content-type", HeaderValue::from_static("application/json"));
    headers
      .insert("user-agent", HeaderValue::from_static(concat!("furb/", env!("CARGO_PKG_VERSION"))));
    let client = openai::Client::builder()
      .api_key(credential.access)
      .base_url(auth::API)
      .http_client(Amended::new(CodexHttp(self.config.clone())))
      .http_headers(headers)
      .build()
      .map_err(CompletionError::HttpError)?;
    let mut settings =
      request.additional_params.take().and_then(|one| one.as_object().cloned()).unwrap_or_default();
    settings.insert("store".to_owned(), Value::Bool(false));
    settings.insert("include".to_owned(), json!(["reasoning.encrypted_content"]));
    settings.insert("text".to_owned(), json!({"format": {"type": "text"}, "verbosity": "low"}));
    request.additional_params = Some(Value::Object(settings));
    request.max_tokens = None;
    client.completion_model(&self.id).stream(request).await
  }
}

/// Codex can stream Responses events without a Content-Type header, and a refusal of its
/// credential asks for a new sign-in. It holds the folder of the config that keeps the sign-in.
#[derive(Clone, Debug, Default)]
struct CodexHttp(PathBuf);

impl Amend for CodexHttp {
  fn streamed(&self, mut response: StreamingResponse) -> http_client::Result<StreamingResponse> {
    if matches!(response.status().as_u16(), 401 | 403) {
      auth::needs_login(&self.0);
      return Err(http_client::Error::InvalidStatusCodeWithMessage(
        response.status(),
        "Sign in to codex again from Providers.".to_owned(),
      ));
    }
    if response.status().is_success() && !response.headers().contains_key("content-type") {
      response.headers_mut().insert("content-type", HeaderValue::from_static("text/event-stream"));
    }
    Ok(response)
  }
}

#[cfg(test)]
#[path = "mod.test.rs"]
mod test;

#[derive(Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
  #[serde(rename = "enabled")]
  _enabled: Option<bool>,
  model_context_window: Option<u64>,
}

pub(super) fn offered(
  config: Option<&Path>,
  form: &Value,
) -> Result<Vec<super::Model>, catalog::Error> {
  let settings: Settings = if *form == Value::Bool(true) {
    Settings::default()
  } else {
    serde_json::from_value(form.clone()).map_err(catalog::Error::CodexSettings)?
  };
  if settings.model_context_window == Some(0) {
    return Err(catalog::Error::CodexWindow);
  }
  let directory = auth::directory(config);
  Ok(
    auth::models(config)
      .into_iter()
      .map(|one| {
        let window = settings
          .model_context_window
          .map(|window| one.max_context_window.map_or(window, |maximum| window.min(maximum)))
          .or(one.context_window)
          .or(one.max_context_window)
          .unwrap_or_default()
          .saturating_mul(one.effective_context_window_percent.unwrap_or(95).min(100))
          / 100;
        // Codex says no price, so the catalog gives the class of each model of its account.
        let class = catalog::family(NAME, &one.slug);
        let mut model = super::Model::new(
          format!("{NAME}:{}", one.slug),
          window.max(1),
          Codex { config: directory.clone(), id: one.slug },
        )
        .conversation("prompt_cache_key");
        if let Some(class) = class {
          model = model.classed(class);
        }
        model.images = one.input_modalities.iter().any(|one| one == "image");
        for effort in one.supported_reasoning_levels {
          if catalog::LEVELS.contains(&effort.effort.as_str()) {
            model = model.effort(
              &effort.effort,
              json!({"reasoning": {"effort": effort.effort, "summary": "auto"}}),
            );
          }
        }
        model
      })
      .collect(),
  )
}
