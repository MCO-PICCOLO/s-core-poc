/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use clap::Parser;
use rocksdb::{IteratorMode, Options, WriteBatch, DB};
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{error, info};

// Import protobuf definitions
use common::rocksdbservice::{
    rocks_db_service_server::{RocksDbService, RocksDbServiceServer},
    BatchPutRequest, BatchPutResponse, DeleteRequest, DeleteResponse, GetByPrefixRequest,
    GetByPrefixResponse, GetRequest, GetResponse, HealthRequest, HealthResponse, KeyValue,
    ListKeysRequest, ListKeysResponse, PutRequest, PutResponse,
};

// Global RocksDB instance
static DB_INSTANCE: OnceLock<Arc<Mutex<DB>>> = OnceLock::new();

#[derive(Parser)]
#[command(name = "rocksdbservice")]
#[command(about = "Pullpiri RocksDB gRPC Service")]
struct Args {
    /// RocksDB data path
    #[arg(short, long, default_value = "/tmp/pullpiri_shared_rocksdb")]
    path: String,

    /// Service port
    #[arg(short = 'P', long, default_value = "47007")]
    port: u16,

    /// Bind address
    #[arg(short, long, default_value = "0.0.0.0")]
    addr: String,
}

// Initialize RocksDB
fn init_db(path: &str) -> anyhow::Result<()> {
    info!("Initializing RocksDB at path: '{}'", path);

    let mut opts = Options::default();
    opts.create_if_missing(true);

    // Performance optimizations
    opts.set_max_background_jobs(6); // 백그라운드 작업 증가
    opts.set_write_buffer_size(128 * 1024 * 1024); // 128MB 버퍼
    opts.set_max_write_buffer_number(4); // 버퍼 개수
    opts.set_target_file_size_base(128 * 1024 * 1024); // SST 파일 크기
    opts.increase_parallelism(6); // CPU 병렬 처리

    // Compression
    opts.set_compression_type(rocksdb::DBCompressionType::Lz4);

    info!("Opening RocksDB with optimized settings...");

    let db = DB::open(&opts, path)?;

    DB_INSTANCE
        .set(Arc::new(Mutex::new(db)))
        .map_err(|_| anyhow::anyhow!("RocksDB already initialized"))?;

    info!("RocksDB successfully initialized at path: '{}'", path);
    Ok(())
}

// Get DB instance safely
fn get_db() -> Result<Arc<Mutex<DB>>, Status> {
    DB_INSTANCE
        .get()
        .ok_or_else(|| Status::unavailable("RocksDB not initialized"))
        .map(|db| db.clone())
}

// gRPC service implementation
pub struct RocksDbServiceImpl;

#[tonic::async_trait]
impl RocksDbService for RocksDbServiceImpl {
    async fn health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        let db_initialized = DB_INSTANCE.get().is_some();

        let status = if db_initialized {
            // Try a simple read operation
            match get_db() {
                Ok(db) => match db.try_lock() {
                    Ok(_) => "healthy".to_string(),
                    Err(_) => "busy".to_string(),
                },
                Err(_) => "error".to_string(),
            }
        } else {
            "error".to_string()
        };

        let response = HealthResponse {
            status,
            version: "1.0.0".to_string(),
            database_path: "/tmp/pullpiri_shared_rocksdb".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn put(&self, request: Request<PutRequest>) -> Result<Response<PutResponse>, Status> {
        let req = request.into_inner();

        // Validate key
        if req.key.is_empty() {
            return Err(Status::invalid_argument("Key cannot be empty"));
        }

        if req.key.len() > 1024 {
            return Err(Status::invalid_argument(
                "Key exceeds maximum allowed length of 1024 characters",
            ));
        }

        if req.key.contains(['<', '>', '?', '{', '}']) {
            return Err(Status::invalid_argument(
                "Key contains invalid special characters",
            ));
        }

        let db = get_db()?;
        let db_lock = db.lock().await;

        match db_lock.put(req.key.as_bytes(), req.value.as_bytes()) {
            Ok(()) => {
                info!("Successfully stored key: '{}'", req.key);
                Ok(Response::new(PutResponse {
                    success: true,
                    error: String::new(),
                }))
            }
            Err(e) => {
                error!("Failed to store key '{}': {}", req.key, e);
                Err(Status::internal(format!("RocksDB put error: {}", e)))
            }
        }
    }

    async fn get(&self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        let req = request.into_inner();

        if req.key.is_empty() {
            return Err(Status::invalid_argument("Key cannot be empty"));
        }

        let db = get_db()?;
        let db_lock = db.lock().await;

        match db_lock.get(req.key.as_bytes()) {
            Ok(Some(value)) => match String::from_utf8(value) {
                Ok(value_str) => {
                    info!("Successfully retrieved key: '{}'", req.key);
                    Ok(Response::new(GetResponse {
                        success: true,
                        value: value_str,
                        message: "Key found".to_string(),
                    }))
                }
                Err(e) => {
                    error!("UTF-8 conversion error for key '{}': {}", req.key, e);
                    Err(Status::internal(format!("UTF-8 conversion error: {}", e)))
                }
            },
            Ok(None) => {
                info!("Key not found: '{}'", req.key);
                Ok(Response::new(GetResponse {
                    success: false,
                    value: String::new(),
                    message: "Key not found".to_string(),
                }))
            }
            Err(e) => {
                error!("Failed to get key '{}': {}", req.key, e);
                Err(Status::internal(format!("RocksDB get error: {}", e)))
            }
        }
    }

    async fn delete(
        &self,
        request: Request<DeleteRequest>,
    ) -> Result<Response<DeleteResponse>, Status> {
        let req = request.into_inner();

        if req.key.is_empty() {
            return Err(Status::invalid_argument("Key cannot be empty"));
        }

        let db = get_db()?;
        let db_lock = db.lock().await;

        match db_lock.delete(req.key.as_bytes()) {
            Ok(()) => {
                info!("Successfully deleted key: '{}'", req.key);
                Ok(Response::new(DeleteResponse {
                    success: true,
                    error: String::new(),
                }))
            }
            Err(e) => {
                error!("Failed to delete key '{}': {}", req.key, e);
                Err(Status::internal(format!("RocksDB delete error: {}", e)))
            }
        }
    }

    async fn batch_put(
        &self,
        request: Request<BatchPutRequest>,
    ) -> Result<Response<BatchPutResponse>, Status> {
        let req = request.into_inner();

        if req.pairs.is_empty() {
            return Ok(Response::new(BatchPutResponse {
                success: true,
                processed_count: 0,
                error: String::new(),
            }));
        }

        // Validate all keys first
        for item in &req.pairs {
            if item.key.is_empty()
                || item.key.len() > 1024
                || item.key.contains(['<', '>', '?', '{', '}'])
            {
                return Err(Status::invalid_argument(format!(
                    "Invalid key: {}",
                    item.key
                )));
            }
        }

        let db = get_db()?;
        let db_lock = db.lock().await;
        let mut batch = WriteBatch::default();

        for item in &req.pairs {
            batch.put(item.key.as_bytes(), item.value.as_bytes());
        }

        match db_lock.write(batch) {
            Ok(()) => {
                info!("Successfully stored {} items in batch", req.pairs.len());
                Ok(Response::new(BatchPutResponse {
                    success: true,
                    processed_count: req.pairs.len() as i32,
                    error: String::new(),
                }))
            }
            Err(e) => {
                error!("Batch write failed: {}", e);
                Err(Status::internal(format!(
                    "RocksDB batch write error: {}",
                    e
                )))
            }
        }
    }

    async fn get_by_prefix(
        &self,
        request: Request<GetByPrefixRequest>,
    ) -> Result<Response<GetByPrefixResponse>, Status> {
        let req = request.into_inner();

        if req.prefix.is_empty() {
            return Err(Status::invalid_argument("Prefix cannot be empty"));
        }

        let db = get_db()?;
        let db_lock = db.lock().await;
        let mut results = Vec::new();
        let iter = db_lock.iterator(IteratorMode::Start);

        for item in iter {
            match item {
                Ok((key_bytes, value_bytes)) => {
                    match (
                        String::from_utf8(key_bytes.to_vec()),
                        String::from_utf8(value_bytes.to_vec()),
                    ) {
                        (Ok(key), Ok(value)) => {
                            if key.starts_with(&req.prefix) {
                                results.push(KeyValue { key, value });
                            }
                        }
                        _ => continue, // Skip invalid UTF-8 entries
                    }
                }
                Err(_) => continue, // Skip errors
            }
        }

        let count = results.len() as i32;
        info!("Found {} keys with prefix '{}'", count, req.prefix);
        Ok(Response::new(GetByPrefixResponse {
            pairs: results,
            total_count: count,
            error: String::new(),
        }))
    }

    async fn list_keys(
        &self,
        request: Request<ListKeysRequest>,
    ) -> Result<Response<ListKeysResponse>, Status> {
        let req = request.into_inner();

        let db = get_db()?;
        let db_lock = db.lock().await;
        let mut keys = Vec::new();
        let iter = db_lock.iterator(IteratorMode::Start);

        let mut count = 0;
        let limit = if req.limit > 0 {
            req.limit as usize
        } else {
            usize::MAX
        };

        for item in iter {
            if count >= limit {
                break;
            }

            match item {
                Ok((key_bytes, _)) => {
                    if let Ok(key) = String::from_utf8(key_bytes.to_vec()) {
                        if req.prefix.is_empty() || key.starts_with(&req.prefix) {
                            keys.push(key);
                            count += 1;
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        let count = keys.len() as i32;
        info!("Listed {} keys with prefix '{}'", count, req.prefix);
        Ok(Response::new(ListKeysResponse {
            keys,
            total_count: count,
            error: String::new(),
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    // Initialize RocksDB
    init_db(&args.path)?;

    let bind_addr = format!("{}:{}", args.addr, args.port).parse()?;
    let rocksdb_service = RocksDbServiceImpl;

    info!("🚀 RocksDB gRPC Service starting on {}", bind_addr);
    info!("📁 Database path: {}", args.path);
    info!("🔗 gRPC endpoint: grpc://{}", bind_addr);

    // Start the gRPC server
    Server::builder()
        .add_service(RocksDbServiceServer::new(rocksdb_service))
        .serve(bind_addr)
        .await?;

    Ok(())
}
