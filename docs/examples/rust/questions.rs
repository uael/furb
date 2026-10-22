use std::{error::Error, io::Write};

use furb::life::Life;
use futures::executor::block_on;

fn main() -> Result<(), Box<dyn Error>> {
  block_on(async {
    let mut life = Life::open("deploy.jsonl").await?;
    let deploy = life.prompt::<String>("Deploy version 2. Ask me before you deploy.")?;
    while let Some(question) = life.question(&deploy).await? {
      print!("{} ", question.message);
      std::io::stdout().flush()?;
      let mut line = String::new();
      std::io::stdin().read_line(&mut line)?;
      question.answer(&mut life, line.trim())?;
    }
    println!("{}", life.wait(deploy).await?);
    Ok(())
  })
}
