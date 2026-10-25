//! The Responses API of a ChatGPT subscription, on the sign-in that furb keeps.

use std::{future::Future, path::PathBuf};

use bytes::Bytes;
use rig_core::{
  client::CompletionClient,
  completion::{CompletionError, CompletionModel, CompletionRequest, CompletionResponse},
  http_client::{
    HeaderMap, HeaderValue, HttpClientExt, LazyBody, MultipartForm, Request, ReqwestClient,
    Response, StreamingResponse,
  },
  providers::openai,
  streaming::StreamingCompletionResponse,
  wasm_compat::WasmCompatSend,
};
use serde_json::{Value, json};

mod auth;

pub use auth::{NAME, Status, begin, cancel, refresh, status};

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
      .http_client(CodexHttp(ReqwestClient::default(), Some(self.config.clone())))
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

/// Codex can stream Responses events without a Content-Type header.
#[derive(Clone, Debug, Default)]
struct CodexHttp(ReqwestClient, Option<PathBuf>);

impl HttpClientExt for CodexHttp {
  fn send<T, U>(
    &self,
    request: Request<T>,
  ) -> impl Future<Output = rig_core::http_client::Result<Response<LazyBody<U>>>>
  + WasmCompatSend
  + 'static
  where
    T: Into<Bytes> + WasmCompatSend,
    U: From<Bytes> + WasmCompatSend + 'static,
  {
    self.0.send(request)
  }

  fn send_multipart<U>(
    &self,
    request: Request<MultipartForm>,
  ) -> impl Future<Output = rig_core::http_client::Result<Response<LazyBody<U>>>>
  + WasmCompatSend
  + 'static
  where
    U: From<Bytes> + WasmCompatSend + 'static,
  {
    self.0.send_multipart(request)
  }

  async fn send_streaming<T>(
    &self,
    request: Request<T>,
  ) -> rig_core::http_client::Result<StreamingResponse>
  where
    T: Into<Bytes> + WasmCompatSend,
  {
    let mut response = self.0.send_streaming(request).await?;
    if matches!(response.status().as_u16(), 401 | 403) {
      if let Some(config) = &self.1 {
        auth::needs_login(config);
      }
      return Err(rig_core::http_client::Error::InvalidStatusCodeWithMessage(
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
  enabled: Option<bool>,
  model_context_window: Option<u64>,
}

pub(super) fn offered(
  config: Option<&std::path::Path>,
  form: &serde_json::Value,
) -> Result<Vec<super::Model>, String> {
  let settings: Settings = if *form == serde_json::Value::Bool(true) {
    Settings::default()
  } else {
    serde_json::from_value(form.clone()).map_err(|no| format!("Invalid codex settings: {no}"))?
  };
  let _ = settings.enabled;
  if settings.model_context_window == Some(0) {
    return Err("The codex model_context_window must be positive".to_owned());
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
        let mut model = super::Model::new(
          format!("{NAME}:{}", one.slug),
          window.max(1),
          Codex { config: directory.clone(), id: one.slug },
        )
        .conversation("prompt_cache_key");
        model.images = one.input_modalities.iter().any(|one| one == "image");
        for effort in one.supported_reasoning_levels {
          if super::catalog::LEVELS.contains(&effort.effort.as_str()) {
            model = model.effort(
              &effort.effort,
              serde_json::json!({"reasoning": {"effort": effort.effort, "summary": "auto"}}),
            );
          }
        }
        model
      })
      .collect(),
  )
}
