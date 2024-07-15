use lancedb::{connect, Result};

#[tokio::main]
async fn main() -> Result<()> {
  let uri = "data/sample-lancedb";
  let db_builder = connect(uri);
  let db_connect = connect(uri).execute().await?;
  println!("Lancedb get started!");
  println!("LanceDB builder: {:?}", db_builder);
  println!("LanceDB connect: {}", db_connect);
  Ok(())
}
