use hipcortex::plugin_host::PluginHost;

#[test]
fn wasm_echo() {
    let host = PluginHost::new();
    #[cfg(feature = "plugin")]
    {
        let wat = "(module (func (export \"run\") (result i32) i32.const 42))";
        let bytes = wat::parse_str(wat).unwrap();
        let result = host.run_wasm(&bytes).unwrap();
        assert_eq!(result, 42);
    }
    #[cfg(not(feature = "plugin"))]
    {
        assert!(host.run_wasm(&[]).is_err());
    }
}
