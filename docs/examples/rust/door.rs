use std::error::Error;

use furb::life::Life;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
  let mut life = Life::open("door.jsonl").await?;
  let release = life.prompt::<String>("Release version 2 in one hour.")?;
  life.step(&release).await?;
  let plan = life.plan(&release)?;
  println!("{plan}");
  life.edit(&release, &plan.replace("await wait(3600)\n", ""))?;
  println!("{}", life.wait(release).await?);
  Ok(())
}
