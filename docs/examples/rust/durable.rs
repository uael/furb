use std::error::Error;

use furb::life::Life;
use futures::executor::block_on;

fn main() -> Result<(), Box<dyn Error>> {
  block_on(async {
    let mut life = Life::open("release.jsonl").await?;
    let plan = life.prompt::<String>("Plan the release of version 2.")?;
    println!("{}", life.wait(plan).await?);
    Ok(())
  })
}
