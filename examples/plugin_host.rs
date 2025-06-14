use hipcortex::plugin_host::PluginHost;

fn main() -> anyhow::Result<()> {
    let host = PluginHost::new();
    #[cfg(feature = "plugin")]
    {
        let wat = "(module (func (export \"run\") (result i32) i32.const 7))";
        let bytes = wat::parse_str(wat)?;
        let result = host.run_wasm(&bytes)?;
        println!("Plugin returned: {}", result);
    }
    #[cfg(not(feature = "plugin"))]
    {
        if let Err(e) = host.run_wasm(&[]) {
            println!("{}", e);
        }
    }
    Ok(())
}
