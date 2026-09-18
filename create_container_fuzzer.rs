#![no_main]

use libfuzzer_sys::fuzz_target;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{Mutex as StdMutex, OnceLock};
use std::sync::Arc;
use std::panic::AssertUnwindSafe;
use std::time::Instant;
use slog::Drain;
use sha2::{Sha256, Digest};
use protobuf::Message;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use protocols::agent::CreateContainerRequest;
use protocols::agent_ttrpc_async::AgentService as TtrpcAgentService;
use kata_agent::sandbox::Sandbox;
use kata_agent::rpc::AgentService;

static HARNESS_STATE: OnceLock<HarnessState> = OnceLock::new();

struct HarnessState {
    logger: slog::Logger,
    csv_writer: StdMutex<File>,
    rt: tokio::runtime::Runtime,
}

pub struct KataFuzzer {
    pub logger: slog::Logger,
}

impl KataFuzzer {
    pub fn rt() -> &'static tokio::runtime::Runtime {
        &HARNESS_STATE.get().unwrap().rt
    }

    pub fn init() -> Self {
        let state = HARNESS_STATE.get_or_init(|| {
            let fuzzer_name = env!("CARGO_BIN_NAME");
            
            let script_bytes = include_bytes!("create_container_fuzzer.rs");
            let mut hasher = Sha256::new();
            hasher.update(script_bytes);
            let script_hash = hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect::<String>();
            
            let timestamp = chrono::Local::now().format("%Y%d%m_%H%M%S");
            let file_prefix = format!("{}_{}_{}", fuzzer_name, &script_hash[..8], timestamp);
            
            let mut csv_file = OpenOptions::new()
                .create(true).write(true).append(true)
                .open(format!("{}.csv", file_prefix)).unwrap();
            
            let _ = writeln!(csv_file, "payload_size,exec_status,info,exec_time_us,seed_base64");

            let log_file = OpenOptions::new()
                .create(true).write(true).append(true)
                .open(format!("{}.log", file_prefix)).unwrap();
                
            let file_decorator = slog_term::PlainSyncDecorator::new(log_file);
            let file_drain = slog_term::FullFormat::new(file_decorator).build().fuse();

            let term_decorator = slog_term::PlainSyncDecorator::new(std::io::stdout());
            let term_drain = slog_term::FullFormat::new(term_decorator).build().fuse();

            let dual_drain = slog::Duplicate::new(file_drain, term_drain).fuse();
            let sync_drain = StdMutex::new(dual_drain).fuse();
            let logger = slog::Logger::root(sync_drain, slog::o!("fuzzer" => fuzzer_name.to_string()));

            let scope_guard = slog_scope::set_global_logger(logger.clone());
            std::mem::forget(scope_guard); 

            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            HarnessState {
                logger,
                csv_writer: StdMutex::new(csv_file),
                rt,
            }
        });

        Self { logger: state.logger.clone() }
    }

    pub fn record_csv(&self, size: usize, status: &str, info: &str, time_us: u128, seed_b64: &str) {
        if let Some(state) = HARNESS_STATE.get() {
            if let Ok(mut csv) = state.csv_writer.lock() {
                let _ = writeln!(csv, "{},{},{},{},{}", size, status, info, time_us, seed_b64);
            }
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() { return; }
    let start_time = Instant::now();

    let Ok(create_req) = CreateContainerRequest::parse_from_bytes(data) else { return; };
    let container_id = create_req.container_id.clone();
    
    let harness = KataFuzzer::init();
    let rt = KataFuzzer::rt();

    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _guard = rt.enter();

        rt.block_on(async {
            let Ok(sandbox) = Sandbox::new(&harness.logger) else { return; };
            let sandbox_arc = Arc::new(tokio::sync::Mutex::new(sandbox));
            let agent_service = AgentService::new_for_fuzzing(sandbox_arc);

            let ctx = ttrpc::r#async::TtrpcContext {
                mh: Default::default(),
                metadata: Default::default(),
                timeout_nano: 0,
            };

            let _ = TtrpcAgentService::create_container(&agent_service, &ctx, create_req).await;
        });
    }));

    let exec_time_us = start_time.elapsed().as_micros();
    let mut seed_base64 = BASE64_STANDARD.encode(data);
    if seed_base64.len() > 128 { seed_base64.truncate(128); }

    harness.record_csv(data.len(), "executed", &container_id, exec_time_us, &seed_base64);
});
