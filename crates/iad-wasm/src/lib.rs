//! WASM plugin runtime for IAD (Phase 4.8-4.9).
//!
//! Runs untrusted agent WASM modules in sandboxed wasmtime engine.
//! Each invocation gets a fresh Store with resource limits.
//!
//! WASM agent ABI:
//!   export fn iad_init()              -> i32   -- 0 = ok
//!   export fn iad_alloc(len: i32)     -> i32   -- returns ptr
//!   export fn iad_handle(ptr, len)    -> i32   -- process task JSON
//!   export fn iad_result_ptr()        -> i32   -- result buffer ptr
//!   export fn iad_result_len()        -> i32   -- result buffer len
//!   import  fn iad::log(ptr, len)              -- host logging

use anyhow::Context;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use wasmtime::*;
use wasmtime_wasi::{WasiCtxBuilder, add_to_linker_async};

// ─── Sandbox limits ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SandboxLimits {
    pub max_memory_bytes:   usize,
    pub fuel:               Option<u64>,
    pub max_table_elements: usize,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes:   16 * 1024 * 1024, // 16 MiB
            fuel:               Some(1_000_000_000),
            max_table_elements: 10_000,
        }
    }
}

// ─── Shared engine ────────────────────────────────────────────────────────────

static ENGINE: std::sync::OnceLock<Engine> = std::sync::OnceLock::new();

fn engine() -> &'static Engine {
    ENGINE.get_or_init(|| {
        let mut cfg = Config::new();
        cfg.async_support(true);
        cfg.consume_fuel(true);
        Engine::new(&cfg).expect("wasmtime engine")
    })
}

// ─── Loaded module ────────────────────────────────────────────────────────────

pub struct WasmPlugin {
    pub name:   String,
    module:     Module,
    pub limits: SandboxLimits,
}

impl WasmPlugin {
    pub fn from_file(name: &str, path: &Path, limits: SandboxLimits) -> anyhow::Result<Self> {
        let module = Module::from_file(engine(), path)
            .with_context(|| format!("loading {}", path.display()))?;
        info!(name, "WASM plugin loaded");
        Ok(Self { name: name.into(), module, limits })
    }

    pub fn from_bytes(name: &str, bytes: &[u8], limits: SandboxLimits) -> anyhow::Result<Self> {
        let module = Module::from_binary(engine(), bytes)
            .context("compiling WASM")?;
        info!(name, bytes = bytes.len(), "WASM plugin loaded from bytes");
        Ok(Self { name: name.into(), module, limits })
    }
}

// ─── Store data ───────────────────────────────────────────────────────────────

struct HostState {
    wasi: wasmtime_wasi::WasiCtx,
    tbl:  wasmtime_wasi::ResourceTable,
}

// ─── Task execution ───────────────────────────────────────────────────────────

pub async fn run_task(plugin: &WasmPlugin, task_json: &str) -> anyhow::Result<String> {
    let wasi = WasiCtxBuilder::new().inherit_stderr().build();
    let tbl  = wasmtime_wasi::ResourceTable::new();
    let mut store = Store::new(engine(), HostState { wasi, tbl });

    if let Some(fuel) = plugin.limits.fuel {
        store.set_fuel(fuel)?;
    }

    let mut linker: Linker<HostState> = Linker::new(engine());
    add_to_linker_async(&mut linker, |s: &mut HostState| wasmtime_wasi::WasiImpl(s))?;

    // Host: iad::log
    linker.func_wrap("iad", "log", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| {
        if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
            let mut buf = vec![0u8; len as usize];
            if mem.read(&caller, ptr as usize, &mut buf).is_ok() {
                tracing::info!(plugin = "wasm", "{}", String::from_utf8_lossy(&buf));
            }
        }
    })?;

    let instance = linker.instantiate_async(&mut store, &plugin.module).await
        .context("WASM instantiation")?;
    let memory = instance.get_memory(&mut store, "memory")
        .context("WASM must export 'memory'")?;

    // iad_init
    if let Ok(f) = instance.get_typed_func::<(), i32>(&mut store, "iad_init") {
        let r = f.call_async(&mut store, ()).await?;
        anyhow::ensure!(r == 0, "iad_init returned {r}");
    }

    // Write task JSON
    let task_bytes = task_json.as_bytes();
    let ptr = if let Ok(alloc) = instance.get_typed_func::<i32, i32>(&mut store, "iad_alloc") {
        alloc.call_async(&mut store, task_bytes.len() as i32).await?
    } else { 1024 };
    memory.write(&mut store, ptr as usize, task_bytes).context("write task")?;

    // iad_handle
    let handle = instance.get_typed_func::<(i32, i32), i32>(&mut store, "iad_handle")
        .context("WASM must export 'iad_handle(ptr,len)->i32'")?;
    let r = handle.call_async(&mut store, (ptr, task_bytes.len() as i32)).await?;
    anyhow::ensure!(r == 0, "iad_handle returned {r}");

    // Read result
    let rptr = instance.get_typed_func::<(), i32>(&mut store, "iad_result_ptr")
        .context("must export iad_result_ptr")?
        .call_async(&mut store, ()).await? as usize;
    let rlen = instance.get_typed_func::<(), i32>(&mut store, "iad_result_len")
        .context("must export iad_result_len")?
        .call_async(&mut store, ()).await? as usize;

    let mut buf = vec![0u8; rlen];
    memory.read(&store, rptr, &mut buf).context("read result")?;
    debug!(plugin = plugin.name, rlen, "WASM task done");
    String::from_utf8(buf).context("WASM result not UTF-8")
}

// ─── Registry ────────────────────────────────────────────────────────────────

pub struct WasmRegistry {
    plugins: Mutex<std::collections::HashMap<String, Arc<WasmPlugin>>>,
}

impl WasmRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { plugins: Mutex::new(std::collections::HashMap::new()) })
    }

    pub async fn load_file(&self, name: &str, path: &Path, limits: SandboxLimits) -> anyhow::Result<()> {
        let p = WasmPlugin::from_file(name, path, limits)?;
        self.plugins.lock().await.insert(name.into(), Arc::new(p));
        Ok(())
    }

    pub async fn load_bytes(&self, name: &str, bytes: &[u8], limits: SandboxLimits) -> anyhow::Result<()> {
        let p = WasmPlugin::from_bytes(name, bytes, limits)?;
        self.plugins.lock().await.insert(name.into(), Arc::new(p));
        Ok(())
    }

    pub async fn get(&self, name: &str) -> Option<Arc<WasmPlugin>> {
        self.plugins.lock().await.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<String> {
        self.plugins.lock().await.keys().cloned().collect()
    }

    pub async fn unload(&self, name: &str) -> bool {
        self.plugins.lock().await.remove(name).is_some()
    }
}

impl Default for WasmRegistry {
    fn default() -> Self {
        Self { plugins: Mutex::new(std::collections::HashMap::new()) }
    }
}
