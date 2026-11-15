use furb_wire::protocol::BOUND;

use crate::{Client, Outgoing};

#[test]
fn each_image_of_the_input_takes_the_next_name_and_a_message_names_it_as_its_life_does() {
  let mut client = Client::default();
  let marks = [client.paste_image(b"one", "png"), client.paste_image(b"two", "jpg")];
  assert_eq!(marks, [Ok("▣ image-1.png".to_owned()), Ok("▣ image-2.jpg".to_owned())]);
  let mut images = client.draft.images.clone();
  images[0].named = Some("![image-1.png](furb-image://a.png)".to_owned());
  let text = "see ▣ image-1.png and ▣ image-2.jpg".to_owned();
  let sent = Outgoing { at: 0, life: "a".to_owned(), thread: None, text, python: false, images };
  assert_eq!(sent.said(), "see ![image-1.png](furb-image://a.png) and ▣ image-2.jpg");
}

#[test]
fn an_image_that_no_frame_holds_stays_out_of_the_input_and_says_why() {
  let mut client = Client::default();
  let refused = client.paste_image(&vec![0; BOUND], "png");
  assert_eq!(refused, Err("An image of at most 12 MiB goes to a session.".to_owned()));
  assert!(client.draft.images.is_empty());
}
