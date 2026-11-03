use std::error::Error;

use furb::life::Life;

#[expect(
  clippy::disallowed_methods,
  reason = "main blocks on the runtime that tokio starts for it"
)]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
  let mut life = Life::open("release.jsonl").await?;
  let plan = life.prompt::<String>("Plan the release of version 2.")?;
  println!("{}", life.wait(plan).await?);
  Ok(())
}
