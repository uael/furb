//! The clients of rig that ask the providers of the catalog: each client that rig or a crate of rig
//! gives, the chat completions of `OpenAI` at the address of a provider or of a local server, and
//! the protocol of pi-ai.

use std::future::Future;

use bytes::Bytes;
use futures::stream;
use rig_core::{
  client::{CompletionClient, ProviderClient},
  completion::{AssistantContent, CompletionModel},
  http_client::{
    self, HeaderMap, HeaderValue, HttpClientExt, LazyBody, MultipartForm, Request, ReqwestClient,
    Response, StreamingResponse,
  },
  providers::{
    anthropic, azure, copilot, deepseek, gemini, groq, huggingface, mistral, moonshot, openai,
    openrouter, together, xai, xiaomimimo, zai,
  },
  streaming::{RawStreamingChoice, StreamingCompletionResponse},
  wasm_compat::WasmCompatSend,
};

use super::{Streams, ended, pi::Pi, streams};
use crate::world::cache;

/// The client of rig that asks a provider.
#[derive(serde::Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub(super) enum Client {
  Anthropic,
  Openai,
  Gemini,
  Openrouter,
  Groq,
  Xai,
  Mistral,
  Deepseek,
  Together,
  Moonshot,
  Zai,
  Huggingface,
  Xiaomi,
  /// The chat completions of `OpenAI` at the address of the provider, which take no credential when
  /// it is empty, as a local server may take none.
  Chat,
  /// The chat completions of the gateway of Cloudflare, which takes the credential of Cloudflare alone.
  Gateway,
  Copilot,
  Azure,
  Bedrock,
  Vertex,
  /// The protocol of pi-ai itself.
  Pi,
}

/// Where a model is asked, and with what: the client of rig, the credential and whether it goes as a bearer, the
/// address, the name the provider knows the model by, the version of the API of Azure, and the project and the
/// location of Google.
#[derive(Clone, Default)]
pub(super) struct Reach {
  pub(super) key: String,
  pub(super) bearer: bool,
  pub(super) api: Option<String>,
  pub(super) id: String,
  pub(super) version: Option<String>,
  pub(super) project: Option<(String, String)>,
}

/// Why a client of rig cannot be made. Each cause holds its text, so that the error clones into the
/// cell that keeps the client of a model.
#[derive(Clone, Debug, thiserror::Error)]
pub(super) enum Error {
  #[error("the provider has no address")]
  NoAddress,
  #[error("{0}")]
  Client(String),
  #[error("{0}")]
  Header(String),
  #[error("{0}")]
  Bedrock(String),
  #[error("{0}")]
  Vertex(String),
  #[error("{0}")]
  Unreached(String),
}

impl From<http_client::Error> for Error {
  fn from(no: http_client::Error) -> Self {
    Self::Client(no.to_string())
  }
}

/// The value that the credential of a client of rig takes when the provider takes the credential under another
/// header, and which [`Unsent`] leaves out of each request.
const UNSENT: &str = "unsent";

impl Client {
  /// What streams the responses of a model through this client.
  pub(super) fn streams(self, reach: &Reach) -> Result<Streams, Error> {
    let (key, id) = (reach.key.clone(), reach.id.clone());
    macro_rules! asked {
      ($client:ty) => {
        <$client>::builder().api_key(key).build()?.completion_model(id)
      };
    }
    let api = || reach.api.clone().ok_or(Error::NoAddress);
    Ok(match self {
      Self::Anthropic if reach.bearer => bearer_streams(reach, &key, id)?,
      Self::Anthropic => {
        let mut built = anthropic::Client::builder().api_key(key);
        if let Some(api) = &reach.api {
          built = built.base_url(api);
        }
        streams(built.build()?.completion_model(id).with_automatic_caching())
      }
      Self::Openai => {
        let mut built = openai::Client::builder().api_key(key);
        if let Some(api) = &reach.api {
          built = built.base_url(api);
        }
        streams(built.build()?.completion_model(id))
      }
      Self::Chat if key.is_empty() => {
        let built = openai::CompletionsClient::builder().api_key(UNSENT).base_url(api()?);
        streams(built.http_client(Amended::new(Unsent)).build()?.completion_model(id))
      }
      Self::Chat => {
        let built = openai::CompletionsClient::builder().api_key(key).base_url(api()?);
        streams(built.build()?.completion_model(id))
      }
      Self::Gateway => {
        let built = openai::CompletionsClient::builder().api_key(UNSENT).base_url(api()?);
        let built = built.http_headers(headed("cf-aig-authorization", &format!("Bearer {key}"))?);
        streams(built.http_client(Amended::new(Unsent)).build()?.completion_model(id))
      }
      Self::Gemini => streams(asked!(gemini::Client)),
      Self::Openrouter => streams(asked!(openrouter::Client)),
      Self::Groq => streams(asked!(groq::Client)),
      Self::Xai => streams(asked!(xai::Client)),
      Self::Mistral => streams(asked!(mistral::Client)),
      Self::Deepseek => streams(asked!(deepseek::Client)),
      Self::Together => streams(asked!(together::Client)),
      Self::Moonshot => streams(asked!(moonshot::Client)),
      Self::Zai => streams(asked!(zai::Client)),
      Self::Huggingface => streams(asked!(huggingface::Client)),
      Self::Xiaomi => streams(asked!(xiaomimimo::Client)),
      Self::Copilot => copilot_streams(key, id)?,
      Self::Azure => azure_streams(reach, key, id)?,
      Self::Bedrock => {
        let built =
          rig_bedrock::client::Client::from_env().map_err(|no| Error::Bedrock(no.to_string()))?;
        streams(built.completion_model(id))
      }
      Self::Vertex => vertex_streams(reach, key, id)?,
      Self::Pi => streams(Pi::new(api()?, key, id)),
    })
  }
}

/// The responses of a model of Anthropic, with a bearer token that the provider takes under the
/// header of
/// `authorization`, in the place of the key of rig.
fn bearer_streams(reach: &Reach, key: &str, id: String) -> Result<Streams, Error> {
  let mut built = anthropic::Client::builder().api_key(UNSENT).http_client(Amended::new(Unsent));
  built = built.http_headers(headed("authorization", &format!("Bearer {key}"))?);
  if let Some(api) = &reach.api {
    built = built.base_url(api);
  }
  Ok(streams(built.build()?.completion_model(id).with_automatic_caching()))
}

/// The responses of a model that GitHub Copilot serves, on the token of its sign-in.
fn copilot_streams(key: String, id: String) -> Result<Streams, Error> {
  let built = copilot::Client::builder().github_access_token(key).allow_device_flow(false);
  let built = match cache() {
    Some(at) => built.token_dir(at.join("copilot")),
    None => built,
  };
  Ok(streams(built.build()?.completion_model(id)))
}

/// The responses of a model that an Azure endpoint serves.
fn azure_streams(reach: &Reach, key: String, id: String) -> Result<Streams, Error> {
  let built = azure::Client::builder().api_key(azure::AzureOpenAIAuth::ApiKey(key));
  let built = built.azure_endpoint(reach.api.clone().ok_or(Error::NoAddress)?);
  let built = built.api_version(reach.version.as_deref().unwrap_or("2024-10-21"));
  Ok(streams(built.build()?.completion_model(id)))
}

/// The responses of a model that Vertex AI serves, whole, since it does not stream.
fn vertex_streams(reach: &Reach, key: String, id: String) -> Result<Streams, Error> {
  let (project, location) = reach.project.clone().unwrap_or_default();
  let mut built = rig_vertexai::Client::builder().with_project(&project).with_location(&location);
  if !key.is_empty() {
    let credentials = google_cloud_auth::credentials::api_key_credentials::Builder::new(key);
    built = built.with_credentials(credentials.build());
  }
  Ok(whole(built.build().map_err(|no| Error::Vertex(no.to_string()))?.completion_model(id)))
}

/// A map of one header.
fn headed(name: &'static str, value: &str) -> Result<HeaderMap, Error> {
  let mut headers = HeaderMap::new();
  headers.insert(name, HeaderValue::from_str(value).map_err(|no| Error::Header(no.to_string()))?);
  Ok(headers)
}

/// What streams the responses of a model of rig that answers each whole: the text of a response streams at once.
fn whole<M: CompletionModel + 'static>(model: M) -> Streams {
  let model = std::sync::Arc::new(model);
  std::sync::Arc::new(move |request| {
    let model = std::sync::Arc::clone(&model);
    Box::pin(async move {
      let response = model.completion(request).await?;
      let texts = response.choice.iter().filter_map(|one| match one {
        AssistantContent::Text(text) => Some(Ok(RawStreamingChoice::Message(text.text.clone()))),
        _ => None,
      });
      let mut parts: Vec<_> = texts.collect();
      parts.push(Ok(RawStreamingChoice::FinalResponse(ended("vertex", response))));
      Ok(StreamingCompletionResponse::stream("vertex", Box::pin(stream::iter(parts))))
    })
  })
}

/// What an [`Amended`] client changes on its way: the request before it goes, and the streamed
/// reply before it is read. A method that a type leaves out changes nothing.
pub(super) trait Amend: Clone + std::fmt::Debug + Send + Sync + 'static {
  fn request<T>(&self, request: Request<T>) -> Request<T> {
    request
  }

  fn streamed(&self, response: StreamingResponse) -> http_client::Result<StreamingResponse> {
    Ok(response)
  }
}

/// A client of HTTP that forwards each call to reqwest with what its [`Amend`] changed.
#[derive(Clone, Debug, Default)]
pub(super) struct Amended<A>(ReqwestClient, A);

impl<A: Amend> Amended<A> {
  pub(super) fn new(amend: A) -> Self {
    Self(ReqwestClient::default(), amend)
  }
}

impl<A: Amend> HttpClientExt for Amended<A> {
  fn send<T, U>(
    &self,
    request: Request<T>,
  ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
  where
    T: Into<Bytes> + WasmCompatSend,
    U: From<Bytes> + WasmCompatSend + 'static,
  {
    self.0.send(self.1.request(request))
  }

  fn send_multipart<U>(
    &self,
    request: Request<MultipartForm>,
  ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
  where
    U: From<Bytes> + WasmCompatSend + 'static,
  {
    self.0.send_multipart(self.1.request(request))
  }

  async fn send_streaming<T>(&self, request: Request<T>) -> http_client::Result<StreamingResponse>
  where
    T: Into<Bytes> + WasmCompatSend,
  {
    self.1.streamed(self.0.send_streaming(self.1.request(request)).await?)
  }
}

/// Leaves out each header whose value ends with [`UNSENT`], which the key of a client of rig holds
/// when the provider takes its credential under another header.
#[derive(Clone, Debug, Default)]
pub(super) struct Unsent;

impl Amend for Unsent {
  fn request<T>(&self, mut request: Request<T>) -> Request<T> {
    let unsent: Vec<_> = (request.headers().iter())
      .filter(|(_, value)| value.as_bytes().ends_with(UNSENT.as_bytes()))
      .map(|(name, _)| name.clone())
      .collect();
    for name in unsent {
      request.headers_mut().remove(name);
    }
    request
  }
}
