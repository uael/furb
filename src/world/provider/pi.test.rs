//! The stream of pi-ai, read from chunks that a test gives, so no test asks the network.

use std::collections::VecDeque;

use bytes::Bytes;
use rig_core::streaming::RawStreamingChoice;

use super::Heard;

#[test]
fn a_letter_that_two_chunks_of_the_stream_split_is_read_whole() {
  let events = "data: {\"type\":\"text_delta\",\"delta\":\"a\"}\r\n\r\n\
                data: {\"type\":\"text_delta\",\"delta\":\"\u{e9}\"}\r\n\r\n\
                data: {\"type\":\"done\",\"usage\":{}}\r\n\r\n";
  let cut = events.find('\u{e9}').expect("the letter") + 1;
  let chunks = [&events.as_bytes()[..cut], &events.as_bytes()[cut..]];
  let body = futures::stream::iter(chunks.map(|one| Ok(Bytes::copy_from_slice(one))));
  let (buffer, parts) = (Vec::new(), VecDeque::new());
  let mut heard = Heard { body: Box::pin(body), buffer, parts, over: false };
  let mut said = Vec::new();
  while let Some(part) = futures::executor::block_on(heard.next()) {
    if let Ok(RawStreamingChoice::Message(text)) = part {
      said.push(text);
    }
  }
  assert_eq!(said, ["a", "\u{e9}"]);
}
