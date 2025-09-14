#[cfg(feature = "plugin")]
use hipcortex::plugin_host::PluginHost;

#[cfg(feature = "plugin")]
#[test]
fn wasm_echo() {
    let host = PluginHost::new();
    let wat = "(module (func (export \"run\") (result i32) i32.const 42))";
    let bytes = wat::parse_str(wat).unwrap();
    let result = host.run_wasm(&bytes).unwrap();
    assert_eq!(result, 42);
}

#[cfg(feature = "plugin")]
#[test]
fn no_export() {
    let host = PluginHost::new();
    let wat = "(module)";
    let bytes = wat::parse_str(wat).unwrap();
    let result = host.run_wasm(&bytes);
    assert!(result.is_err());
}
