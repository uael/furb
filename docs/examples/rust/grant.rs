use std::error::Error;

use furb::life::Life;

#[expect(
  clippy::disallowed_methods,
  reason = "main blocks on the runtime that tokio starts for it"
)]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
  let mut life = Life::open("budget.jsonl").await?;
  let budget = life.chain("budget")?;
  life.grant(&budget, 0.50)?;
  let count = life.prompt_on::<i64>(&budget, "Count the lines of python in this directory.")?;
  life.paused(&budget).await?;
  println!("paused at {:.2} dollars", life.spent(&budget)?);
  life.grant(&budget, 2.00)?;
  life.wake(&budget)?;
  println!("{}", life.wait(count).await?);
  Ok(())
}
