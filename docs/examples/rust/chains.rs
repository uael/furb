#![expect(clippy::expect_used, reason = "debt: an expect where a type can hold the invariant")]

use std::error::Error;

use furb::life::Life;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
  let mut life = Life::open("notes.jsonl").roster(["claude-cli:opus", "claude-cli:sonnet"]).await?;
  let reviewer = life.chain("reviewer")?;
  life.model(&reviewer, "claude-cli:sonnet")?;
  let draft = life.prompt::<String>("Write the release notes of version 2.")?;
  let draft = life.wait(draft).await?;
  let review =
    life.prompt_on::<String>(&reviewer, &format!("Review these release notes:\n{draft}"))?;
  println!("{}", life.wait(review).await?);
  for chain in life.chains()? {
    let parent = life.parent(&chain)?.expect("a chain under the root");
    println!("{} under {}", life.label(&chain)?, life.label(&parent)?);
  }
  Ok(())
}
