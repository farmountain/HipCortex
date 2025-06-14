use hipcortex::plugin_host::PluginHost;

#[test]
fn user_executes_plugin() {
    let host = PluginHost::new();
    #[cfg(feature = "plugin")]
    {
        let wat = "(module (func (export \"run\") (result i32) i32.const 3))";
        let bytes = wat::parse_str(wat).unwrap();
        let result = host.run_wasm(&bytes).unwrap();
        assert_eq!(result, 3);
    }
    #[cfg(not(feature = "plugin"))]
    {
        assert!(host.run_wasm(&[]).is_err());
    }
}
