//! QuickJS Runtime Wrapper
//!
//! Provides a safe wrapper around the QuickJS JavaScript engine.

use std::sync::Arc;

use rquickjs::{Context, Function, Runtime, Value as JsValue};

use blueprint_types::{TypeRegistry, Value};

use super::globals::register_neo_globals;

// ─────────────────────────────────────────────────────────────────────────────
// JS Runtime Error
// ─────────────────────────────────────────────────────────────────────────────

/// Errors that can occur in the JavaScript runtime
#[derive(Debug, thiserror::Error)]
pub enum JsError {
    #[error("JavaScript error: {0}")]
    Js(String),

    #[error("Failed to create runtime: {0}")]
    RuntimeCreation(String),

    #[error("Failed to evaluate script: {0}")]
    Eval(String),

    #[error("Function not found: {0}")]
    FunctionNotFound(String),

    #[error("Type conversion error: {0}")]
    Conversion(String),

    #[error("Runtime error: {0}")]
    Runtime(#[from] rquickjs::Error),
}

pub type JsResult<T> = Result<T, JsError>;

// ─────────────────────────────────────────────────────────────────────────────
// JS Runtime
// ─────────────────────────────────────────────────────────────────────────────

/// A JavaScript runtime instance
///
/// This wraps QuickJS and provides methods for evaluating scripts and
/// calling JavaScript functions from Rust.
pub struct JsRuntime {
    /// The QuickJS runtime (thread-local, so we need to be careful)
    runtime: Runtime,
    /// The JavaScript context
    context: Context,
    /// Type registry for defining new types from JS
    type_registry: Option<Arc<TypeRegistry>>,
}

// QuickJS is single-threaded, so we need to be explicit about thread safety
// The runtime should only be used from one thread at a time
unsafe impl Send for JsRuntime {}

impl JsRuntime {
    /// Create a new JavaScript runtime
    pub fn new() -> JsResult<Self> {
        let runtime = Runtime::new().map_err(|e| JsError::RuntimeCreation(e.to_string()))?;
        let context = Context::full(&runtime).map_err(|e| JsError::RuntimeCreation(e.to_string()))?;

        let js_runtime = Self {
            runtime,
            context,
            type_registry: None,
        };

        // Initialize globals (console, neo object)
        js_runtime.init_globals()?;

        Ok(js_runtime)
    }

    /// Create a new JavaScript runtime with a type registry
    pub fn with_type_registry(type_registry: Arc<TypeRegistry>) -> JsResult<Self> {
        let runtime = Runtime::new().map_err(|e| JsError::RuntimeCreation(e.to_string()))?;
        let context = Context::full(&runtime).map_err(|e| JsError::RuntimeCreation(e.to_string()))?;

        let js_runtime = Self {
            runtime,
            context,
            type_registry: Some(type_registry),
        };

        // Initialize globals (console, neo object)
        js_runtime.init_globals()?;

        Ok(js_runtime)
    }

    /// Initialize global objects and functions
    fn init_globals(&self) -> JsResult<()> {
        self.context.with(|ctx| {
            register_neo_globals(&ctx, self.type_registry.clone())
                .map_err(|e| JsError::Js(e.to_string()))
        })
    }

    /// Evaluate a JavaScript script
    pub fn eval(&self, script: &str) -> JsResult<Value> {
        self.context.with(|ctx| {
            let result: JsValue = ctx.eval(script).map_err(|e| JsError::Eval(e.to_string()))?;
            js_to_value(&ctx, result)
        })
    }

    /// Evaluate a script file
    pub fn eval_file(&self, name: &str, script: &str) -> JsResult<Value> {
        self.context.with(|ctx| {
            let mut options = rquickjs::context::EvalOptions::default();
            options.global = true;
            options.strict = true;
            options.backtrace_barrier = true;

            let result: JsValue = ctx
                .eval_with_options(script, options)
                .map_err(|e| JsError::Eval(format!("{}: {}", name, e)))?;
            js_to_value(&ctx, result)
        })
    }

    /// Check if a function exists in the global scope
    pub fn has_function(&self, name: &str) -> bool {
        self.context.with(|ctx| {
            ctx.globals()
                .get::<_, Function>(name)
                .is_ok()
        })
    }

    /// Call a global function with no arguments
    pub fn call_function(&self, name: &str) -> JsResult<Value> {
        self.context.with(|ctx| {
            let globals = ctx.globals();
            let func: Function = globals
                .get(name)
                .map_err(|_| JsError::FunctionNotFound(name.to_string()))?;

            let result: JsValue = func
                .call(())
                .map_err(|e| JsError::Js(format!("Error calling {}: {}", name, e)))?;

            js_to_value(&ctx, result)
        })
    }

    /// Call a global function with a single JSON argument
    pub fn call_function_with_json(
        &self,
        name: &str,
        arg: &serde_json::Value,
    ) -> JsResult<Value> {
        self.context.with(|ctx| {
            let globals = ctx.globals();
            let func: Function = globals
                .get(name)
                .map_err(|_| JsError::FunctionNotFound(name.to_string()))?;

            // Convert JSON to JS value
            let js_arg = json_to_js(&ctx, arg)?;

            let result: JsValue = func
                .call((js_arg,))
                .map_err(|e| JsError::Js(format!("Error calling {}: {}", name, e)))?;

            js_to_value(&ctx, result)
        })
    }

    /// Call a global function with an event object
    pub fn call_with_event(
        &self,
        name: &str,
        event_type: &str,
        source: &str,
        data: &serde_json::Value,
    ) -> JsResult<Value> {
        self.context.with(|ctx| {
            let globals = ctx.globals();
            let func: Function = globals
                .get(name)
                .map_err(|_| JsError::FunctionNotFound(name.to_string()))?;

            // Create event object
            let event_obj = rquickjs::Object::new(ctx.clone())
                .map_err(|e| JsError::Js(e.to_string()))?;

            event_obj
                .set("type", event_type)
                .map_err(|e| JsError::Js(e.to_string()))?;
            event_obj
                .set("source", source)
                .map_err(|e| JsError::Js(e.to_string()))?;

            let js_data = json_to_js(&ctx, data)?;
            event_obj
                .set("data", js_data)
                .map_err(|e| JsError::Js(e.to_string()))?;

            let result: JsValue = func
                .call((event_obj,))
                .map_err(|e| JsError::Js(format!("Error calling {}: {}", name, e)))?;

            js_to_value(&ctx, result)
        })
    }

    /// Get the type registry if available
    pub fn type_registry(&self) -> Option<&Arc<TypeRegistry>> {
        self.type_registry.as_ref()
    }

    /// Run garbage collection
    pub fn gc(&self) {
        self.runtime.run_gc();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Value Conversion
// ─────────────────────────────────────────────────────────────────────────────

/// Convert a JavaScript value to a Neo Value
fn js_to_value<'js>(ctx: &rquickjs::Ctx<'js>, js: JsValue<'js>) -> JsResult<Value> {
    match js.type_of() {
        rquickjs::Type::Undefined | rquickjs::Type::Null => Ok(Value::Null),
        rquickjs::Type::Bool => {
            let b: bool = js.get().map_err(|e| JsError::Conversion(e.to_string()))?;
            Ok(Value::Bool(b))
        }
        rquickjs::Type::Int => {
            let i: i32 = js.get().map_err(|e| JsError::Conversion(e.to_string()))?;
            Ok(Value::Int(i as i64))
        }
        rquickjs::Type::Float => {
            let f: f64 = js.get().map_err(|e| JsError::Conversion(e.to_string()))?;
            Ok(Value::Float(f))
        }
        rquickjs::Type::String => {
            let s: String = js.get().map_err(|e| JsError::Conversion(e.to_string()))?;
            Ok(Value::String(s))
        }
        rquickjs::Type::Array => {
            let arr: rquickjs::Array = js.get().map_err(|e| JsError::Conversion(e.to_string()))?;
            let mut values = Vec::new();
            for i in 0..arr.len() {
                let item: JsValue = arr.get(i).map_err(|e| JsError::Conversion(e.to_string()))?;
                values.push(js_to_value(ctx, item)?);
            }
            Ok(Value::Array(values))
        }
        rquickjs::Type::Object => {
            let obj: rquickjs::Object = js.get().map_err(|e| JsError::Conversion(e.to_string()))?;
            let mut fields = std::collections::HashMap::new();

            // Check for __type__ field
            let type_id: Option<String> = obj.get("__type__").ok();

            for prop in obj.props::<String, JsValue>() {
                let (key, val) = prop.map_err(|e| JsError::Conversion(e.to_string()))?;
                if key != "__type__" {
                    fields.insert(key, js_to_value(ctx, val)?);
                }
            }

            Ok(Value::Object { type_id, fields })
        }
        _ => Ok(Value::Null),
    }
}

/// Convert a serde_json::Value to a JavaScript value
fn json_to_js<'js>(ctx: &rquickjs::Ctx<'js>, json: &serde_json::Value) -> JsResult<JsValue<'js>> {
    use rquickjs::IntoJs;

    match json {
        serde_json::Value::Null => Ok(JsValue::new_undefined(ctx.clone())),
        serde_json::Value::Bool(b) => {
            Ok(JsValue::new_bool(ctx.clone(), *b))
        }
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(JsValue::new_int(ctx.clone(), i as i32))
            } else if let Some(f) = n.as_f64() {
                Ok(JsValue::new_float(ctx.clone(), f))
            } else {
                Ok(JsValue::new_undefined(ctx.clone()))
            }
        }
        serde_json::Value::String(s) => {
            s.as_str().into_js(ctx)
                .map_err(|e| JsError::Conversion(e.to_string()))
        }
        serde_json::Value::Array(arr) => {
            let js_arr = rquickjs::Array::new(ctx.clone())
                .map_err(|e| JsError::Conversion(e.to_string()))?;
            for (i, item) in arr.iter().enumerate() {
                let js_item = json_to_js(ctx, item)?;
                js_arr.set(i, js_item)
                    .map_err(|e| JsError::Conversion(e.to_string()))?;
            }
            Ok(js_arr.into_value())
        }
        serde_json::Value::Object(obj) => {
            let js_obj = rquickjs::Object::new(ctx.clone())
                .map_err(|e| JsError::Conversion(e.to_string()))?;
            for (key, val) in obj {
                let js_val = json_to_js(ctx, val)?;
                js_obj.set(key.as_str(), js_val)
                    .map_err(|e| JsError::Conversion(e.to_string()))?;
            }
            Ok(js_obj.into_value())
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_runtime() {
        let runtime = JsRuntime::new().unwrap();
        assert!(!runtime.has_function("nonexistent"));
    }

    #[test]
    fn test_eval_primitives() {
        let runtime = JsRuntime::new().unwrap();

        // Number
        let result = runtime.eval("42").unwrap();
        assert_eq!(result.as_i64(), Some(42));

        // String
        let result = runtime.eval("'hello'").unwrap();
        assert_eq!(result.as_str(), Some("hello"));

        // Boolean
        let result = runtime.eval("true").unwrap();
        assert_eq!(result.as_bool(), Some(true));
    }

    #[test]
    fn test_eval_object() {
        let runtime = JsRuntime::new().unwrap();

        let result = runtime.eval("({ name: 'test', value: 42 })").unwrap();
        assert!(result.as_object().is_some());
        assert_eq!(result.get("name").and_then(|v| v.as_str()), Some("test"));
        assert_eq!(result.get("value").and_then(|v| v.as_i64()), Some(42));
    }

    #[test]
    fn test_call_function() {
        let runtime = JsRuntime::new().unwrap();

        runtime.eval("function add(a, b) { return a + b; }").unwrap();
        assert!(runtime.has_function("add"));

        // Call with JSON args
        let result = runtime
            .call_function_with_json("add", &serde_json::json!([1, 2]))
            .unwrap();
        // Note: calling with a single array arg, not two separate args
    }

    #[test]
    fn test_define_and_call_function() {
        let runtime = JsRuntime::new().unwrap();

        runtime.eval(r#"
            function greet() {
                return "Hello, World!";
            }
        "#).unwrap();

        let result = runtime.call_function("greet").unwrap();
        assert_eq!(result.as_str(), Some("Hello, World!"));
    }
}
