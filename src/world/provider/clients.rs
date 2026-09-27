//! The clients of rig that ask the providers of the catalog: each client that rig or a crate of rig gives, the chat
//! completions of OpenAI at the address of a provider, and the protocol of pi-ai.

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

use super::{Streams, catalog::cache, ended, pi::Pi, streams};

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
  /// The chat completions of OpenAI at the address of the provider.
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

/// The value that the credential of a client of rig takes when the provider takes the credential under another
/// header, and which [`Unsent`] leaves out of each request.
const UNSENT: &str = "unsent";

impl Client {
  /// What streams the responses of a model through this client.
  pub(super) fn streams(self, reach: &Reach) -> Result<Streams, String> {
    let (key, id) = (reach.key.clone(), reach.id.clone());
    let failed = |no: http_client::Error| no.to_string();
    macro_rules! asked {
      ($client:ty) => {
        <$client>::builder().api_key(key).build().map_err(failed)?.completion_model(id)
      };
    }
    let api = || reach.api.clone().ok_or("the provider has no address".to_owned());
    Ok(match self {
      Client::Anthropic if reach.bearer => {
        let mut built = anthropic::Client::builder().api_key(UNSENT).http_client(Unsent::default());
        built = built.http_headers(headed("authorization", &format!("Bearer {key}"))?);
        if let Some(api) = &reach.api {
          built = built.base_url(api);
        }
        streams(built.build().map_err(failed)?.completion_model(id).with_automatic_caching())
      }
      Client::Anthropic => {
        let mut built = anthropic::Client::builder().api_key(key);
        if let Some(api) = &reach.api {
          built = built.base_url(api);
        }
        streams(built.build().map_err(failed)?.completion_model(id).with_automatic_caching())
      }
      Client::Openai => {
        let mut built = openai::Client::builder().api_key(key);
        if let Some(api) = &reach.api {
          built = built.base_url(api);
        }
        streams(built.build().map_err(failed)?.completion_model(id))
      }
      Client::Chat => {
        let built = openai::CompletionsClient::builder().api_key(key).base_url(api()?);
        streams(built.build().map_err(failed)?.completion_model(id))
      }
      Client::Gateway => {
        let built = openai::CompletionsClient::builder().api_key(UNSENT).base_url(api()?);
        let built = built.http_headers(headed("cf-aig-authorization", &format!("Bearer {key}"))?);
        streams(built.http_client(Unsent::default()).build().map_err(failed)?.completion_model(id))
      }
      Client::Gemini => streams(asked!(gemini::Client)),
      Client::Openrouter => streams(asked!(openrouter::Client)),
      Client::Groq => streams(asked!(groq::Client)),
      Client::Xai => streams(asked!(xai::Client)),
      Client::Mistral => streams(asked!(mistral::Client)),
      Client::Deepseek => streams(asked!(deepseek::Client)),
      Client::Together => streams(asked!(together::Client)),
      Client::Moonshot => streams(asked!(moonshot::Client)),
      Client::Zai => streams(asked!(zai::Client)),
      Client::Huggingface => streams(asked!(huggingface::Client)),
      Client::Xiaomi => streams(asked!(xiaomimimo::Client)),
      Client::Copilot => {
        let built = copilot::Client::builder().github_access_token(key).allow_device_flow(false);
        let built = match cache() {
          Some(at) => built.token_dir(at.join("copilot")),
          None => built,
        };
        streams(built.build().map_err(failed)?.completion_model(id))
      }
      Client::Azure => {
        let built = azure::Client::builder().api_key(azure::AzureOpenAIAuth::ApiKey(key));
        let built = built.azure_endpoint(api()?);
        let built = built.api_version(reach.version.as_deref().unwrap_or("2024-10-21"));
        streams(built.build().map_err(failed)?.completion_model(id))
      }
      Client::Bedrock => {
        let built = rig_bedrock::client::Client::from_env().map_err(|no| no.to_string())?;
        streams(built.completion_model(id))
      }
      Client::Vertex => {
        let (project, location) = reach.project.clone().unwrap_or_default();
        let mut built =
          rig_vertexai::Client::builder().with_project(&project).with_location(&location);
        if !key.is_empty() {
          let credentials = google_cloud_auth::credentials::api_key_credentials::Builder::new(key);
          built = built.with_credentials(credentials.build());
        }
        whole(built.build().map_err(|no| no.to_string())?.completion_model(id))
      }
      Client::Pi => streams(Pi::new(api()?, key, id)),
    })
  }
}

/// A map of one header.
fn headed(name: &'static str, value: &str) -> Result<HeaderMap, String> {
  let mut headers = HeaderMap::new();
  headers.insert(name, HeaderValue::from_str(value).map_err(|no| no.to_string())?);
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

/// A client of HTTP that leaves out each header whose value ends with [`UNSENT`], which the key of a client of rig
/// holds when the provider takes its credential under another header.
#[derive(Clone, Debug, Default)]
struct Unsent(ReqwestClient);

impl Unsent {
  /// The request with no header that holds [`UNSENT`].
  fn sent<T>(mut request: Request<T>) -> Request<T> {
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

impl HttpClientExt for Unsent {
  fn send<T, U>(
    &self,
    request: Request<T>,
  ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
  where
    T: Into<Bytes> + WasmCompatSend,
    U: From<Bytes> + WasmCompatSend + 'static,
  {
    self.0.send(Unsent::sent(request))
  }

  fn send_multipart<U>(
    &self,
    request: Request<MultipartForm>,
  ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
  where
    U: From<Bytes> + WasmCompatSend + 'static,
  {
    self.0.send_multipart(Unsent::sent(request))
  }

  fn send_streaming<T>(
    &self,
    request: Request<T>,
  ) -> impl Future<Output = http_client::Result<StreamingResponse>> + WasmCompatSend
  where
    T: Into<Bytes> + WasmCompatSend,
  {
    self.0.send_streaming(Unsent::sent(request))
  }
}
