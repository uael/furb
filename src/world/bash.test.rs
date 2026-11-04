//! The ear of the commands.

use super::Running;

#[test]
fn a_text_fed_to_a_command_that_reads_no_more_is_refused_and_its_close_is_not() {
  let (stdin, fed) = unsync::spsc::unbounded();
  let (stop, _stopped) = unsync::oneshot::channel();
  let mut one = Running { stdin, _stop: stop };
  assert!(one.feed(Some("one\n".to_owned())).is_ok());
  drop(fed);
  let refused = one.feed(Some("two\n".to_owned())).unwrap_err();
  assert_eq!(refused.message(), "the command reads no more");
  assert!(one.feed(None).is_ok());
}
