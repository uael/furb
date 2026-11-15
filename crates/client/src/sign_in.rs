//! The sign-in of Codex, which the operator asks from the models: the account of his `ChatGPT`
//! plan serves the models of `OpenAI`, with no key. It belongs to the user of a machine, so it
//! needs no session. The machine runs it and tells each change of its state; once it connects,
//! each project of the machine offers the models of Codex.

use furb_wire::protocol::{Answer, Frame, Verb};

use crate::{Asked, Client};

/// A sign-in of Codex while it runs: its machine, whether the client asked it, whether the
/// operator canceled it and the client asked the cancel, the state that the machine told last, the
/// address of its page in the browser while it waits for him, and the code that he enters there
/// when he is not at that machine.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignIn {
  pub at: usize,
  pub asked: bool,
  pub canceled: Option<bool>,
  pub state: Option<String>,
  pub url: Option<String>,
  pub code: Option<String>,
}

impl Client {
  /// A sign-in of Codex on the machine of the target of the input, unless one runs.
  pub fn sign_in(&mut self) {
    let at = self.target().machine();
    self.signing.get_or_insert(SignIn { at, ..SignIn::default() });
  }

  /// The sign-in that runs canceled: the machine refuses the sign-in, and turns nothing on.
  pub fn cancel_sign_in(&mut self) {
    if let Some(signing) = self.signing.as_mut().filter(|one| one.asked) {
      signing.canceled.get_or_insert(false);
    }
  }

  /// The sign-in, which the machine answers once it ends, or its cancel. The page of a sign-in on
  /// another machine calls back that machine, not the browser of the operator, so there he signs
  /// in from a device, with a code.
  pub(crate) fn sign_in_due(&mut self) -> Option<(usize, Verb, Asked)> {
    let signing = self.signing.as_mut()?;
    let at = signing.at;
    if !signing.asked {
      signing.asked = true;
      let cwd = (at == 0).then(|| self.folder.clone());
      return Some((at, Verb::SignIn { cwd, device: at != 0 }, Asked::SignIn));
    }
    if signing.canceled == Some(false) {
      signing.canceled = Some(true);
      return Some((at, Verb::CancelSignIn, Asked::Control));
    }
    None
  }

  /// A state that a machine told of the sign-in that runs there, with the address of its page
  /// and the code to enter there while it waits: the address when it is new, which the client
  /// opens once.
  pub fn told_sign_in(
    &mut self,
    at: usize,
    state: String,
    (url, code): (Option<String>, Option<String>),
  ) -> Option<String> {
    let signing = self.signing.as_mut().filter(|one| one.at == at)?;
    let new = url.clone().filter(|one| signing.url.as_ref() != Some(one));
    (signing.state, signing.url, signing.code) = (Some(state), url, code);
    new
  }

  /// What the machine answered to the sign-in: it connected, with the models that it offers now,
  /// those of Codex among them, or the client says why Codex did not connect.
  pub fn signed_in(&mut self, heard: Option<Frame>) {
    let Some(signing) = self.signing.take() else { return };
    self.notice = Some(match heard {
      Some(Frame::Reply { answer, .. }) => {
        if let (Some(told), Answer::Models(models)) = (self.machines.get_mut(signing.at), answer) {
          told.models = Some(models);
        }
        "Codex is connected: the models of your ChatGPT plan are in the list.".to_owned()
      }
      // The machine pushes the state of the sign-in apart from its answer, so the refusal can
      // come before the state that says the cancel.
      Some(Frame::Refused { .. }) if signing.canceled.is_some() => {
        "The sign-in of Codex stopped. Nothing changed.".to_owned()
      }
      Some(Frame::Refused { why, .. }) => format!("Codex did not connect: {why}"),
      _ => "Codex did not connect: its machine went away.".to_owned(),
    });
  }
}
