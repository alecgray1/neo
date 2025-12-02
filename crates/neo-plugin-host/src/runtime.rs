use anyhow::Result;
use deno_core::{extension, JsRuntime, RuntimeOptions};

use crate::ops;

/// Create a new JS runtime with Neo extensions
pub fn create_runtime() -> Result<JsRuntime> {
    let runtime = JsRuntime::new(RuntimeOptions {
        extensions: vec![neo_ext::init_ops_and_esm()],
        ..Default::default()
    });

    Ok(runtime)
}

/// Call a lifecycle function on the plugin
pub async fn call_lifecycle(runtime: &mut JsRuntime, fn_name: &str, payload: &[u8]) -> Result<()> {
    // For now, we'll use JSON for the payload until we integrate V8 ValueSerializer
    // TODO: Switch to V8 binary format
    let payload_json = if payload.is_empty() {
        "undefined".to_string()
    } else {
        String::from_utf8_lossy(payload).to_string()
    };

    let code = format!(
        r#"
        (async () => {{
            if (typeof globalThis.{fn_name} === 'function') {{
                const payload = {payload_json};
                await globalThis.{fn_name}(payload);
            }}
        }})();
        "#
    );

    let result = runtime.execute_script("<lifecycle>", code);
    match result {
        Ok(_) => {
            // Run the event loop to completion
            runtime.run_event_loop(Default::default()).await?;
            Ok(())
        }
        Err(e) => {
            tracing::error!("Error calling {}: {}", fn_name, e);
            Err(anyhow::anyhow!("Error calling {}: {}", fn_name, e))
        }
    }
}

extension!(
    neo_ext,
    ops = [
        // Legacy JSON ops (kept for compatibility)
        ops::op_neo_log,
        ops::op_neo_emit,
        ops::op_point_read,
        ops::op_point_write,
        // V8 binary serialization ops
        ops::op_emit_v8,
        ops::op_log_v8,
        ops::op_point_read_v8,
        ops::op_point_write_v8,
        // Utilities
        ops::op_sleep,
    ],
    esm_entry_point = "ext:neo_ext/bootstrap.js",
    esm = [
        "ext:neo_ext/bootstrap.js" = "src/bootstrap.js"
    ],
);
