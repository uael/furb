use super::{Part, breakdown};

#[test]
fn the_tokens_of_the_prompt_split_by_the_size_of_each_part_and_sum_to_the_prompt() {
  let sizes = [(Part::Messages, 600), (Part::SystemPrompt, 300), (Part::Skills, 100)];
  let got = breakdown(1000, 101, &sizes, 200);
  let tokens: Vec<(Part, u64)> = got.iter().map(|one| (one.part, one.tokens)).collect();
  assert_eq!(
    tokens,
    [
      (Part::Messages, 61),
      (Part::SystemPrompt, 30),
      (Part::Skills, 10),
      (Part::Reserve, 200),
      (Part::Free, 699),
    ]
  );
  assert_eq!(got.iter().map(|one| one.tokens).sum::<u64>(), 1000);
  assert!((got[0].share - 0.061).abs() < 1e-9);
}

#[test]
fn a_reserve_past_the_room_left_takes_only_that_room_and_no_size_gives_no_token() {
  let got = breakdown(100, 90, &[(Part::Messages, 0)], 50);
  let tokens: Vec<(Part, u64)> = got.iter().map(|one| (one.part, one.tokens)).collect();
  assert_eq!(tokens, [(Part::Messages, 0), (Part::Reserve, 10), (Part::Free, 0)]);
  assert!(breakdown(0, 0, &[], 0).iter().all(|one| one.share == 0.0));
}
