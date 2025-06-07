#[cfg(feature = "plugin")]
pub struct PluginHost {
    engine: wasmtime::Engine,
}

#[cfg(feature = "plugin")]
impl PluginHost {
    pub fn new() -> Self {
        Self {
            engine: wasmtime::Engine::default(),
        }
    }

    pub fn run_wasm(&self, bytes: &[u8]) -> anyhow::Result<i32> {
        use wasmtime::{Instance, Module, Store};
        let module = Module::from_binary(&self.engine, bytes)?;
        let mut store = Store::new(&self.engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;
        let run = instance.get_typed_func::<(), i32>(&mut store, "run")?;
        let result = run.call(&mut store, ())?;
        Ok(result)
    }
}

#[cfg(not(feature = "plugin"))]
pub struct PluginHost;

#[cfg(not(feature = "plugin"))]
impl PluginHost {
    pub fn new() -> Self {
        Self
    }
    pub fn run_wasm(&self, _bytes: &[u8]) -> anyhow::Result<i32> {
        Err(anyhow::anyhow!("plugin feature disabled"))
    }
}
