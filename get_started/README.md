# LanceDB Get Started

本文介绍从头开始使用LanceDB，每一个步骤会给出详细的说明

## 环境搭建
- Rust 
- tokio
- lancedb

### Rust安装

``` sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 创建Rust项目

``` sh
cargo new lancedb-example
cd lancedb-example
```

### 安装Tokio

``` sh
cargo add tokio --features rt-multi-thread
```

### 安装LanceDB

``` sh
cargo add lancedb
```

## 代码解析

``` rust
use lancedb::{connect, Result};

#[tokio::main]
async fn main() -> Result<()> {
  uri = "data/example-lancedb";
  db_builder = connect(uri);
  db_connect = connect(uri).execute().await?;
  println!("LanceDB builder: {:}", db_builder);
  println!("LanceDB connect: {}", db_connect);
  Ok(())
}
```
:::note
上述代码创建了一个最简单的LanceDB数据库创建和连接，并且打印`ConnectBuilder`和`Connection`:
- `uri`: 表示数据库的URI
- `connect`: 接收URI参数，返回`ConnectBuilder`, `COnnectBuilder`实现了`Debug` trait
- `execute`: 返回`Connection`, 实现了`Display` trait，会在执行的目录创建
  `data/example-lancedb`目录
:::

### `connect`函数的定义如下

``` rust
/// Connect to a LanceDB database.
///
/// # Arguments
///
/// * `uri` - URI where the database is located, can be a local directory, supported remote cloud storage,
///           or a LanceDB Cloud database.  See [ConnectOptions::uri] for a list of accepted formats
pub fn connect(uri: &str) -> ConnectBuilder {
  ConnectBuilder::new(uri)
}
```
:::note
在`connect`函数中创建了`ConnectBuilder`，调用`connect`即创建了一个`ConnectBuilder`
:::

### `ConnectBuilder`

``` rust
pub struct ConnectBuilder {
  /// Database URI
  ///
  /// ### Accpeted URI formats
  ///
  /// - `/path/to/database` - local database on file system.
  /// - `s3://bucket/path/to/database` or `gs://bucket/path/to/database` - database on cloud object store
  /// - `db://dbname` - LanceDB Cloud
  uri: String,

  /// LanceDB Cloud API key, required if using Lance Cloud
  api_key: Option<String>,
  /// LanceDB Cloud region, required if using Lance Cloud
  region: Option<String>,
  /// LanceDB Cloud host override, only required if using an on-premises Lance Cloud instance
  host_override: Option<String>,

  storage_options: HashMap<String, String>,

  /// The interval at which to check for updates from other processes.
  ///
  /// If None, then consistency is not checked. For performance
  /// reasons, this is the default. For strong consistency, set this to
  /// zero seconds. Then every read will check for updates from other
  /// processes. As a compromise, you can set this to a non-zero timedelta
  /// for eventual consistency. If more than that interval has passed since
  /// the last check, then the table will be checked for updates. Note: this
  /// consistency only applies to read operations. Write operations are
  /// always consistent.
  read_consistency_interval: Option<std::time::Duration>,
  embedding_registry: Option<Arc<dyn EmbeddingRegistry>>,
}

impl ConnectBuilder {
  /// Create a new [`ConnectOptions`] with the given database URI.
  pub fn new(uri: &str) -> Self {
    Self {
      uri: uri.to_string(),
      api_key: None,
      region: None,
      host_override: None,
      read_consistency_interval: None,
      storage_options: HashMap::new(),
      embedding_registry: None,
    }
  }

  // ......
  
  /// Establishes a connection to the database
  pub async fn execute(self) -> Result<Connection> {
    if self.uri.starts_with("db") {
      self.execute_remote()
    } else {
      let internal = Arc::new(Database::connect_with_options(&self).await?);
      Ok(Connection {
           internal,
           uri: self.uri,
      })
    }
  }
}
```
:::note
`ConnectBuilder`是一个结构体，用于配置和建立与LanceDB数据库的连接

作用:

`ConnectBuilder`通过存储连接参数(如URI、API秘钥、区域等)来构建数据库连接。

它提供了一些方法来设置这些参数，并通过`execute`方法建立实际的数据库连接。
:::

主要字段和方法:
- `uri: String`: 数据库的URI
- `api_key: Option<String>`: LanceDB Cloud的API秘钥
- `region: Option<String>`: LanceDB Cloud的区域
- `storage_options: HashMap<String, String>`: 存储选项
- `read_consistency_interval: Option<std::time::Duration>`: 读一致性检查间隔。
- `embedding_registry: Option<Arc<dyn EmbeddingRegistry>>`: 嵌入注册表。

主要方法:
- `new(uri: &str) -> Self`: 创建一个新的`ConnectBuilder`实例
- `execute(self) -> Result<Connection>`: 执行连接建立，返回一个`Connection`实例

`execute`函数:
- `execute`函数根据`uri`以及数据库连接选项，创建数据库连接
- `execute`函数中首先判断`uri`是否是以`db`开头，如果是`db`开头，执行远程执行。
- 否则，使用`Database`设置连接选项，传入`Connection`对象，使用选项和`uri`创建`Connection`

### `Database`

``` rust
#[derive(Debug)]
struct Database {
  object_store: ObjectStore,
  query_string: Option<String>,

  pub(crate) uri: String,
  pub(crate) base_path: object_store::path::Path,

  // the object store wrapper to use on write path
  pub(crate) store_wrapper: Option<Arc<dyn WrappingObjectStore>>,

  read_consistency_interval: Option<std::time::Duration>,

  // Storage options to be inherited by tables created from this connection
  storage_options: HashMap<String, String>,
  embedding_registry: Arc<dyn EmbeddingRegistry>,
}

impl Database {
  async fn connect_with_options(options: &ConnectBuilder) -> Result<Self> {
    let uri = &options.uri;
    let parse_res = url::Url::parse(uri);

    // TODO: pass params regardless of OS
    match parse_res {
      Ok(url) if url.scheme().len() == 1 && cfg!(windows) => {
        Self::open_path(
          uri,
          options.read_consistency_interval,
          options.embedding_registry.clone(),
        )
       .await
     }
     Ok(mut url) => {
       // iter thru the query params and extract the commit store param
       let mut engine = None;
       let mut mirrored_store = None;
       let mut filtered_querys = vec![];

       // WARNING: specifying engine is NOT a publicly supported feature in lancedb yet
       // THE API WILL CHANGE
       for (key, value) in url.query_pairs() {
         if key == ENGINE {
           engine = Some(value.to_string());
         } else if key == MIRRORED_STORE {
           if cfg!(windows) {
             return Err(Error::NotSupported {
               message: "mirrored store is not supported on windows".into(),
             });
           }
           mirrored_store = Some(value.to_string());
         } else {
           // to owned so we can modify the url
           filtered_querys.push((key.to_string(), value.to_string()));
         }
       }

       // Filter out the commit store query param -- it's a lancedb param
       url.query_pairs_mut().clear();
       url.query_pairs_mut().extend_pairs(filtered_querys);
       // Take a copy of the query string so we can propagate it to lance
       let query_string = url.query().map(|s| s.to_string());
       // clear the query string so we can use the url as the base uri
       // use .set_query(None) instead of .set_query("") because the latter
       // will add a trailing '?' to the url
       url.set_query(None);

       let table_base_uri = if let Some(store) = engine {
         static WARN_ONCE: std::sync::Once = std::sync::Once::new();
         WARN_ONCE.call_once(|| {
           log::warn!("Specifing engine is not a publicly supported feature in lancedb yet. THE API WILL CHANGE");
         });
         let old_scheme = url.scheme().to_string();
         let new_scheme = format!("{}+{}", old_scheme, store);
         url.to_string().replacen(&old_scheme, &new_scheme, 1)
       } else {
         url.to_string()
       };

       let plain_uri = url.to_string();

       let storage_options = options.storage_options.clone();
       let os_params = ObjectStoreParams {
         storage_options: Some(storage_options.clone()),
         ..Default::default()
       };
       let (object_store, base_path) =
         ObjectStore::from_uri_and_params(&plain_uri, &os_params).await?;
       if object_store.is_local() {
         Self::try_create_dir(&plain_uri).context(CreateDirSnafu { path: plain_uri })?;
       }

       let write_store_wrapper = match mirrored_store {
         Some(path) => {
           let mirrored_store = Arc::new(LocalFileSystem::new_with_prefix(path)?);
           let wrapper = MirroringObjectStoreWrapper::new(mirrored_store);
           Some(Arc::new(wrapper) as Arc<dyn WrappingObjectStore>)
         }
         None => None,
       };

       let embedding_registry = options
           .embedding_registry
           .clone()
           .unwrap_or_else(|| Arc::new(MemoryRegistry::new()));
       Ok(Self {
         uri: table_base_uri,
         query_string,
         base_path,
         object_store,
         store_wrapper: write_store_wrapper,
         read_consistency_interval: options.read_consistency_interval,
         storage_options,
         embedding_registry,
       })
      }
      Err(_) => {
        Self::open_path(
          uri,
          options.read_consistency_interval,
          options.embedding_registry.clone(),
        )
        .await
      }
    }
  }
}  
```

:::note
`Database`是一个结构体，表示LanceDB数据库实例

作用:

`Database`结构体封装了与实际数据库交互的逻辑。
它负责管理数据库的基本路径、对象存储、嵌入注册表等信息，
并提供方法来连接和操作数据库。
:::

主要字段:
- `object_store: ObjectStore`: 数据库的对象存储。
- `query_string: Option<String>`: 查询字符串。
- `uri: String`: 数据库的URI。
- `base_path: object_store::path::Path`: 基础路径。
- `store_wrapper: Option<Arc<dyn WrappingObjectStore>>`: 存储包装器。
- `read_consistency_interval: Option<std::time::Duration>`: 读一致性检查间隔。
- `storage_options: HashMap<String, String>`: 存储选项。
- `embedding_registry: Arc<dyn EmbeddingRegistry>`: 嵌入注册表。

主要方法:
- `connect_with_options(options: &ConnectBuilder) -> Result<Self>`: 根据`ConnectBuilder`配置建立数据库连接。

### `Connection`

``` rust
/// A connection to LanceDB
#[derive(Clone)]
pub struct Connection {
  uri: String,
  internal: Arc<dyn ConnectionInternal>,
}
```
:::note
`Connection`是一个结构体，表示与LanceDB数据库的连接实例

作用:

`Connection`结构体持有数据库连接的相关信息，并提供与数据库交互的接口。
它是建立在`ConnectBuilder`配置基础上的实际连接对象。
:::

主要字段:
- `uri: String`: 数据库的URI
- `internal: Arc<dyn ConnectionInternal>`: 内部连接实现


## 总结
- `connect`: 一个函数，用于创建`ConnectBuilder`实例，并初始化数据库连接设置。
- `ConnectBuilder`: 一个结构体，负责配置和建立数据库连接。它存储连接参数，并提供`execute`方法来建立实际的数据库连接。
- `Connection`: 一个结构体，表示与LanceDB数据库的连接实例，持有连接信息并提供与数据库交互的接口。
- `Database`: 一个结构体，表示LanceDB数据库实例，封装了与数据库交互的逻辑和相关信息。

## 链接
- [博客地址](https://yuxuetr.com/blog/2024/07/15/lancedb-get-started)

