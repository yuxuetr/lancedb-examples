use arrow_array::types::Float32Type;
use arrow_array::{FixedSizeListArray, Int32Array, RecordBatch, RecordBatchIterator, StringArray};
use arrow_schema::{DataType, Field, Schema};
use lancedb::arrow::IntoArrow;
use lancedb::query::VectorQuery;
use lancedb::{connect, Result, Table as LanceDbTable};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
  let created_empty_table = create_empty_table().await?;
  println!(
    "Created empty table: {}, Table name: {}",
    created_empty_table,
    created_empty_table.name()
  );

  let created_table_with_data = create_table_with_data().await?;
  println!(
    "Created table with data: {}, Table name: {}",
    created_table_with_data,
    created_table_with_data.name()
  );

  // update_table().await?;

  let created_table_with_records = create_table_with_records().await?;
  println!(
    "Created table with records: {}, Table name: {}",
    created_table_with_records,
    created_table_with_records.name()
  );

  let opened_table = open_with_existing_table().await?;
  println!(
    "Opened table: {}, Table name: {}",
    opened_table,
    opened_table.name()
  );

  let queried_result = query_table().await?;
  println!("Query result: {:?}", queried_result);

  delete_table_records().await?; // 根据条件删除表中的记录
  drop_table().await?; // 删除 data/sample-lancedb/my_table
  drop_database().await?; // 删除 data/sample-lancedb
  Ok(())
}

#[allow(unused)]
async fn query_table() -> Result<VectorQuery> {
  let uri = "data/sample-lancedb";
  let db = connect(uri).execute().await?;
  let table = db.open_table("my_table").execute().await?;
  let result = table.query().nearest_to(&[1.0; 128])?;
  Ok(result)
}

#[allow(unused)]
async fn drop_database() -> Result<()> {
  let uri = "data/sample-lancedb";
  let db = connect(uri).execute().await?;
  db.drop_db().await?;
  Ok(())
}

#[allow(unused)]
async fn update_table() -> Result<()> {
  let uri = "data/sample-lancedb";
  let db = connect(uri).execute().await?;
  let table = db.open_table("table_with_person").execute().await?;
  println!("Before update: {:?}", table.query());
  table
    .update()
    .only_if("id=0")
    .column("name", "Bob")
    .execute()
    .await?; // Bob -> Halzzz

  Ok(())
}

#[allow(unused)]
async fn drop_table() -> Result<()> {
  let uri = "data/sample-lancedb";
  let db = connect(uri).execute().await?;
  db.drop_table("my_table").await?;
  Ok(())
}

#[allow(unused)]
async fn delete_table_records() -> Result<()> {
  let uri = "data/sample-lancedb";
  let db = connect(uri).execute().await?;
  let table = db.open_table("my_table").execute().await?;
  table.delete("id > 24").await?;
  Ok(())
}

#[allow(unused)]
async fn open_with_existing_table() -> Result<LanceDbTable> {
  let uri = "data/sample-lancedb";
  let db = connect(uri).execute().await?;
  let table = db.open_table("my_table").execute().await?;
  Ok(table)
}

#[allow(unused)]
async fn create_table_with_records() -> Result<LanceDbTable> {
  let uri = "data/sample-lancedb";
  let db = connect(uri).execute().await?;

  let initial_data = create_some_records()?;
  let tbl = db.create_table("my_table", initial_data).execute().await?;

  let new_data = create_some_records()?;
  // 只有实现了 IntoArrow 的类型才能使用`add`方法，即`create_some_records`返回的类型
  tbl.add(new_data).execute().await?;
  Ok(tbl)
}

#[allow(unused)]
async fn create_table_with_data() -> Result<LanceDbTable> {
  let uri = "data/sample-lancedb";
  let db_conn = connect(uri).execute().await?;

  let schema = Arc::new(Schema::new(vec![
    Field::new("id", DataType::Int32, false),
    Field::new("name", DataType::Utf8, false),
  ]));

  let ids = Int32Array::from(vec![1, 2, 3]);
  let names = StringArray::from(vec!["Alice", "Bob", "Lily"]);
  let batch = RecordBatch::try_new(schema.clone(), vec![Arc::new(ids), Arc::new(names)])?;
  let batchs = RecordBatchIterator::new(vec![batch].into_iter().map(Ok), schema);

  let table = db_conn
    .create_table("table_with_person", batchs)
    .execute()
    .await?;
  Ok(table)
}

#[allow(unused)]
fn create_some_records() -> Result<impl IntoArrow> {
  const TOTAL: usize = 1000;
  const DIM: usize = 128;

  let schema = Arc::new(Schema::new(vec![
    Field::new("id", DataType::Int32, false),
    Field::new(
      "vector",
      DataType::FixedSizeList(
        Arc::new(Field::new("item", DataType::Float32, true)),
        DIM as i32,
      ),
      true,
    ),
  ]));

  let batch = RecordBatch::try_new(
    schema.clone(),
    vec![
      Arc::new(Int32Array::from_iter_values(0..TOTAL as i32)),
      Arc::new(
        FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
          (0..TOTAL).map(|_| Some(vec![Some(1.0); DIM])),
          DIM as i32,
        ),
      ),
    ],
  )
  .unwrap();
  let batches = RecordBatchIterator::new(vec![batch].into_iter().map(Ok), schema.clone());
  Ok(Box::new(batches))
}

#[allow(unused)]
async fn create_empty_table() -> Result<LanceDbTable> {
  let schema = Arc::new(Schema::new(vec![
    Field::new("id", DataType::Int32, false),
    Field::new("name", DataType::Utf8, false),
  ]));
  let uri = "data/sample-lancedb";
  let db_conn = connect(uri).execute().await?;
  let table = db_conn
    .create_empty_table("empty_table", schema)
    .execute()
    .await?;
  Ok(table)
}
