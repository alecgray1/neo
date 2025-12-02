//! JavaScript Global Objects
//!
//! Registers the `neo` global object and its methods in the JavaScript runtime.

use std::sync::Arc;

use rquickjs::{Ctx, Function, Object, Result as JsResult};
use rquickjs::function::Rest;

use blueprint_types::TypeRegistry;

/// Register the `neo` global object and all its methods
pub fn register_neo_globals(ctx: &Ctx<'_>, type_registry: Option<Arc<TypeRegistry>>) -> JsResult<()> {
    let globals = ctx.globals();

    // Create the neo namespace object
    let neo = Object::new(ctx.clone())?;

    // Register console.log if not already present
    register_console(ctx)?;

    // Register neo.log
    neo.set("log", Function::new(ctx.clone(), |msg: String| {
        tracing::info!(target: "neo.js", "{}", msg);
    })?)?;

    // Register neo.debug
    neo.set("debug", Function::new(ctx.clone(), |msg: String| {
        tracing::debug!(target: "neo.js", "{}", msg);
    })?)?;

    // Register neo.warn
    neo.set("warn", Function::new(ctx.clone(), |msg: String| {
        tracing::warn!(target: "neo.js", "{}", msg);
    })?)?;

    // Register neo.error
    neo.set("error", Function::new(ctx.clone(), |msg: String| {
        tracing::error!(target: "neo.js", "{}", msg);
    })?)?;

    // Set the neo object on globals
    globals.set("neo", neo)?;

    Ok(())
}

/// Register the console object with log, warn, error methods
fn register_console(ctx: &Ctx<'_>) -> JsResult<()> {
    let globals = ctx.globals();

    // Check if console already exists
    if globals.get::<_, Object>("console").is_ok() {
        return Ok(());
    }

    let console = Object::new(ctx.clone())?;

    console.set("log", Function::new(ctx.clone(), |args: Rest<String>| {
        let msg = args.0.join(" ");
        println!("{}", msg);
    })?)?;

    console.set("info", Function::new(ctx.clone(), |args: Rest<String>| {
        let msg = args.0.join(" ");
        tracing::info!(target: "neo.js.console", "{}", msg);
    })?)?;

    console.set("warn", Function::new(ctx.clone(), |args: Rest<String>| {
        let msg = args.0.join(" ");
        tracing::warn!(target: "neo.js.console", "{}", msg);
    })?)?;

    console.set("error", Function::new(ctx.clone(), |args: Rest<String>| {
        let msg = args.0.join(" ");
        tracing::error!(target: "neo.js.console", "{}", msg);
    })?)?;

    console.set("debug", Function::new(ctx.clone(), |args: Rest<String>| {
        let msg = args.0.join(" ");
        tracing::debug!(target: "neo.js.console", "{}", msg);
    })?)?;

    globals.set("console", console)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rquickjs::{Context, Runtime};

    #[test]
    fn test_register_globals() {
        let runtime = Runtime::new().unwrap();
        let ctx = Context::full(&runtime).unwrap();

        ctx.with(|ctx| {
            register_neo_globals(&ctx, None).unwrap();

            // Check neo object exists
            let globals = ctx.globals();
            let neo: Object = globals.get("neo").unwrap();

            // Check neo.log exists
            let _log: Function = neo.get("log").unwrap();
        });
    }

    #[test]
    fn test_console_log() {
        let runtime = Runtime::new().unwrap();
        let ctx = Context::full(&runtime).unwrap();

        ctx.with(|ctx| {
            register_neo_globals(&ctx, None).unwrap();

            // This should not panic
            let _: () = ctx.eval("console.log('Hello from JS')").unwrap();
        });
    }
}
