//! Blueprint Macros - Proc macros for node registration and documentation
//!
//! This crate provides the `#[blueprint_node]` attribute macro for
//! registering nodes with automatic documentation extraction.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Expr, ItemFn, Lit, Meta, Token,
};

/// Parsed attributes for the blueprint_node macro
struct BlueprintNodeAttrs {
    id: String,
    name: Option<String>,
    category: String,
    pure: bool,
    latent: bool,
}

impl Parse for BlueprintNodeAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut id = None;
        let mut name = None;
        let mut category = None;
        let mut pure = false;
        let mut latent = false;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "id" => {
                    let lit: Lit = input.parse()?;
                    if let Lit::Str(s) = lit {
                        id = Some(s.value());
                    }
                }
                "name" => {
                    let lit: Lit = input.parse()?;
                    if let Lit::Str(s) = lit {
                        name = Some(s.value());
                    }
                }
                "category" => {
                    let lit: Lit = input.parse()?;
                    if let Lit::Str(s) = lit {
                        category = Some(s.value());
                    }
                }
                "pure" => {
                    let lit: Lit = input.parse()?;
                    if let Lit::Bool(b) = lit {
                        pure = b.value();
                    }
                }
                "latent" => {
                    let lit: Lit = input.parse()?;
                    if let Lit::Bool(b) = lit {
                        latent = b.value();
                    }
                }
                _ => {
                    return Err(syn::Error::new(ident.span(), "unknown attribute"));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(BlueprintNodeAttrs {
            id: id.ok_or_else(|| input.error("missing required attribute 'id'"))?,
            name,
            category: category.ok_or_else(|| input.error("missing required attribute 'category'"))?,
            pure,
            latent,
        })
    }
}

/// Extract doc comments from attributes
fn extract_doc_comment(attrs: &[syn::Attribute]) -> Option<String> {
    let docs: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                if let Meta::NameValue(meta) = &attr.meta {
                    if let Expr::Lit(expr_lit) = &meta.value {
                        if let Lit::Str(s) = &expr_lit.lit {
                            return Some(s.value().trim().to_string());
                        }
                    }
                }
            }
            None
        })
        .collect();

    if docs.is_empty() {
        None
    } else {
        Some(docs.join("\n"))
    }
}

/// Derive display name from node ID (e.g., "neo/Add" -> "Add")
fn derive_name(id: &str) -> String {
    id.split('/').last().unwrap_or(id).to_string()
}

/// Attribute macro for registering blueprint nodes.
///
/// This macro generates registration code and extracts documentation from
/// doc comments.
///
/// # Attributes
///
/// - `id` (required): Unique node identifier (e.g., "neo/Add")
/// - `category` (required): Node category (e.g., "Math")
/// - `name` (optional): Display name (defaults to last part of ID)
/// - `pure` (optional): Whether this is a pure node (default: false)
/// - `latent` (optional): Whether this node can suspend (default: false)
///
/// # Example
///
/// ```ignore
/// /// Add two numbers together.
/// ///
/// /// This node takes two real inputs and produces their sum.
/// #[blueprint_node(id = "neo/Add", category = "Math", pure = true)]
/// fn add_node(ctx: &mut NodeContext) -> NodeOutput {
///     let a = ctx.get_input_real("a").unwrap_or(0.0);
///     let b = ctx.get_input_real("b").unwrap_or(0.0);
///     let mut values = HashMap::new();
///     values.insert("result".to_string(), Value::from(a + b));
///     NodeOutput::pure(values)
/// }
/// ```
///
/// This generates:
/// - The original function
/// - `add_node_def() -> NodeDef` - returns the node definition
/// - `add_node_description() -> Option<String>` - returns the doc comment
/// - `register_add_node(registry: &mut NodeRegistry)` - registers the node
#[proc_macro_attribute]
pub fn blueprint_node(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as BlueprintNodeAttrs);
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let fn_block = &input_fn.block;
    let fn_inputs = &input_fn.sig.inputs;
    let fn_output = &input_fn.sig.output;
    let fn_attrs = &input_fn.attrs;

    // Extract doc comment
    let doc_comment = extract_doc_comment(fn_attrs);

    // Generate names
    let node_id = &attrs.id;
    let node_name = attrs.name.unwrap_or_else(|| derive_name(node_id));
    let node_category = &attrs.category;
    let node_pure = attrs.pure;
    let node_latent = attrs.latent;

    // Generate helper function names
    let def_fn_name = format_ident!("{}_def", fn_name);
    let desc_fn_name = format_ident!("{}_description", fn_name);
    let register_fn_name = format_ident!("register_{}", fn_name);

    // Generate description expression
    let description_expr = match doc_comment {
        Some(doc) => quote! { Some(#doc.to_string()) },
        None => quote! { None },
    };

    let output = quote! {
        // Re-emit doc comments for IDE support
        #(#fn_attrs)*
        #fn_vis fn #fn_name(#fn_inputs) #fn_output #fn_block

        /// Returns the node definition for this node.
        ///
        /// Note: Pin definitions must be provided when calling this function.
        /// Use `register_` function for full registration.
        #fn_vis fn #def_fn_name(pins: Vec<blueprint_types::PinDef>) -> blueprint_types::NodeDef {
            blueprint_types::NodeDef {
                id: #node_id.to_string(),
                name: #node_name.to_string(),
                category: #node_category.to_string(),
                pure: #node_pure,
                latent: #node_latent,
                pins,
                description: #description_expr,
            }
        }

        /// Returns the documentation for this node.
        #fn_vis fn #desc_fn_name() -> Option<String> {
            #description_expr
        }

        /// Registers this node with the given registry.
        ///
        /// Note: You must provide the pin definitions.
        #fn_vis fn #register_fn_name(
            registry: &mut blueprint_runtime::NodeRegistry,
            pins: Vec<blueprint_types::PinDef>,
        ) {
            let def = #def_fn_name(pins);
            registry.register_fn(def, #fn_name);
        }
    };

    output.into()
}

/// Macro for generating node documentation at build time.
///
/// This is a helper macro that can be used to generate documentation
/// files from node definitions.
#[proc_macro]
pub fn generate_node_docs(_input: TokenStream) -> TokenStream {
    // This would generate documentation files
    // For now, just a placeholder
    TokenStream::new()
}

// ─────────────────────────────────────────────────────────────────────────────
// #[neo::expose] - Universal FFI Macro
// ─────────────────────────────────────────────────────────────────────────────

/// Parsed attributes for the expose macro
struct ExposeAttrs {
    /// Name to expose as (defaults to function name)
    name: Option<String>,
    /// Whether to expose to JavaScript
    js: bool,
    /// Whether to expose to Blueprints
    blueprint: bool,
    /// Whether to expose to TypeScript types
    typescript: bool,
    /// Category for blueprint nodes
    category: Option<String>,
    /// Whether this is a pure function (no side effects)
    pure: bool,
}

impl Default for ExposeAttrs {
    fn default() -> Self {
        Self {
            name: None,
            js: true,
            blueprint: true,
            typescript: true,
            category: None,
            pure: false,
        }
    }
}

impl Parse for ExposeAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = ExposeAttrs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;

            match ident.to_string().as_str() {
                "name" => {
                    input.parse::<Token![=]>()?;
                    let lit: Lit = input.parse()?;
                    if let Lit::Str(s) = lit {
                        attrs.name = Some(s.value());
                    }
                }
                "js" => {
                    input.parse::<Token![=]>()?;
                    let lit: Lit = input.parse()?;
                    if let Lit::Bool(b) = lit {
                        attrs.js = b.value();
                    }
                }
                "blueprint" => {
                    input.parse::<Token![=]>()?;
                    let lit: Lit = input.parse()?;
                    if let Lit::Bool(b) = lit {
                        attrs.blueprint = b.value();
                    }
                }
                "typescript" => {
                    input.parse::<Token![=]>()?;
                    let lit: Lit = input.parse()?;
                    if let Lit::Bool(b) = lit {
                        attrs.typescript = b.value();
                    }
                }
                "category" => {
                    input.parse::<Token![=]>()?;
                    let lit: Lit = input.parse()?;
                    if let Lit::Str(s) = lit {
                        attrs.category = Some(s.value());
                    }
                }
                "pure" => {
                    input.parse::<Token![=]>()?;
                    let lit: Lit = input.parse()?;
                    if let Lit::Bool(b) = lit {
                        attrs.pure = b.value();
                    }
                }
                _ => {
                    return Err(syn::Error::new(ident.span(), format!("unknown attribute: {}", ident)));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(attrs)
    }
}

/// Convert a Rust type to a blueprint PinType representation
fn rust_type_to_pin_type(ty: &syn::Type) -> (String, String) {
    let type_str = quote!(#ty).to_string().replace(' ', "");

    // Map common Rust types to PinType variants
    let (pin_type, ts_type) = match type_str.as_str() {
        "bool" => ("Bool", "boolean"),
        "i8" | "i16" | "i32" | "i64" | "isize" => ("Int", "number"),
        "u8" | "u16" | "u32" | "u64" | "usize" => ("Int", "number"),
        "f32" | "f64" => ("Real", "number"),
        "String" | "&str" => ("String", "string"),
        "Vec<bool>" => ("Array(Box::new(PinType::Bool))", "boolean[]"),
        "Vec<i32>" | "Vec<i64>" => ("Array(Box::new(PinType::Int))", "number[]"),
        "Vec<f32>" | "Vec<f64>" => ("Array(Box::new(PinType::Real))", "number[]"),
        "Vec<String>" => ("Array(Box::new(PinType::String))", "string[]"),
        "()" => ("Flow", "void"),
        _ => {
            // Check for Option<T>
            if type_str.starts_with("Option<") {
                let inner = &type_str[7..type_str.len() - 1];
                let (inner_pin, inner_ts) = match inner {
                    "bool" => ("Bool", "boolean"),
                    "i32" | "i64" => ("Int", "number"),
                    "f32" | "f64" => ("Real", "number"),
                    "String" => ("String", "string"),
                    _ => ("Any", "any"),
                };
                return (inner_pin.to_string(), format!("{} | null", inner_ts));
            }
            // Default to Any for unknown types
            ("Any", "any")
        }
    };

    (pin_type.to_string(), ts_type.to_string())
}

/// Attribute macro for exposing Rust functions to JavaScript, Blueprints, and TypeScript.
///
/// This macro generates:
/// - JavaScript bindings for QuickJS
/// - Blueprint node registration
/// - TypeScript type definitions
///
/// # Attributes
///
/// - `name` (optional): Name to expose as (defaults to function name)
/// - `js` (optional): Whether to expose to JavaScript (default: true)
/// - `blueprint` (optional): Whether to expose to Blueprints (default: true)
/// - `typescript` (optional): Whether to expose to TypeScript (default: true)
/// - `category` (optional): Category for blueprint nodes
/// - `pure` (optional): Whether this is a pure function (default: false)
///
/// # Example
///
/// ```ignore
/// /// Calculate the square of a number.
/// #[neo::expose(category = "Math", pure = true)]
/// fn square(x: f64) -> f64 {
///     x * x
/// }
/// ```
///
/// This generates:
/// - `square(x: f64) -> f64` - The original function
/// - `square_js_binding()` - QuickJS binding generator
/// - `square_blueprint_def()` - Blueprint node definition
/// - `square_typescript_def()` - TypeScript type definition
/// - `register_square(...)` - Registration function for all targets
#[proc_macro_attribute]
pub fn expose(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as ExposeAttrs);
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let fn_block = &input_fn.block;
    let fn_inputs = &input_fn.sig.inputs;
    let fn_output = &input_fn.sig.output;
    let fn_attrs = &input_fn.attrs;

    // Extract doc comment
    let doc_comment = extract_doc_comment(fn_attrs);

    // Determine the exposed name
    let exposed_name = attrs.name.unwrap_or_else(|| fn_name.to_string());
    let category = attrs.category.unwrap_or_else(|| "Uncategorized".to_string());

    // Generate helper function names
    let js_binding_fn = format_ident!("{}_js_binding", fn_name);
    let blueprint_def_fn = format_ident!("{}_blueprint_def", fn_name);
    let typescript_def_fn = format_ident!("{}_typescript_def", fn_name);
    let register_fn = format_ident!("register_{}", fn_name);

    // Parse function parameters
    let mut param_names = Vec::new();
    let mut param_types = Vec::new();
    let mut pin_defs = Vec::new();
    let mut ts_params = Vec::new();

    for input in fn_inputs.iter() {
        if let syn::FnArg::Typed(pat_type) = input {
            if let syn::Pat::Ident(ident) = &*pat_type.pat {
                let param_name = ident.ident.to_string();
                let (pin_type_str, ts_type) = rust_type_to_pin_type(&pat_type.ty);

                param_names.push(param_name.clone());
                param_types.push(pin_type_str.clone());
                ts_params.push(format!("{}: {}", param_name, ts_type));

                pin_defs.push(quote! {
                    blueprint_types::PinDef {
                        id: #param_name.to_string(),
                        name: #param_name.to_string(),
                        direction: blueprint_types::PinDirection::Input,
                        pin_type: blueprint_types::PinType::Any, // TODO: proper type mapping
                        default_value: None,
                    }
                });
            }
        }
    }

    // Parse return type
    let (return_pin_type, ts_return_type) = match fn_output {
        syn::ReturnType::Default => ("Flow".to_string(), "void".to_string()),
        syn::ReturnType::Type(_, ty) => rust_type_to_pin_type(ty),
    };

    // Add output pin
    if return_pin_type != "Flow" {
        pin_defs.push(quote! {
            blueprint_types::PinDef {
                id: "result".to_string(),
                name: "Result".to_string(),
                direction: blueprint_types::PinDirection::Output,
                pin_type: blueprint_types::PinType::Any, // TODO: proper type mapping
                default_value: None,
            }
        });
    }

    // Generate TypeScript definition string
    let ts_params_str = ts_params.join(", ");
    let ts_def = format!(
        "declare function {}({}): {};",
        exposed_name, ts_params_str, ts_return_type
    );

    // Generate description expression
    let description_expr = match &doc_comment {
        Some(doc) => quote! { Some(#doc.to_string()) },
        None => quote! { None },
    };

    let is_pure = attrs.pure;
    let gen_js = attrs.js;
    let gen_blueprint = attrs.blueprint;
    let gen_typescript = attrs.typescript;

    // Generate the output
    let output = quote! {
        // Re-emit the original function with doc comments
        #(#fn_attrs)*
        #fn_vis fn #fn_name(#fn_inputs) #fn_output #fn_block

        /// Returns metadata about the exposed function for JavaScript binding.
        #fn_vis fn #js_binding_fn() -> (&'static str, bool) {
            (#exposed_name, #gen_js)
        }

        /// Returns the Blueprint node definition for this function.
        #fn_vis fn #blueprint_def_fn() -> Option<blueprint_types::NodeDef> {
            if !#gen_blueprint {
                return None;
            }

            Some(blueprint_types::NodeDef {
                id: format!("neo/{}", #exposed_name),
                name: #exposed_name.to_string(),
                category: #category.to_string(),
                pure: #is_pure,
                latent: false,
                pins: vec![#(#pin_defs),*],
                description: #description_expr,
            })
        }

        /// Returns the TypeScript type definition for this function.
        #fn_vis fn #typescript_def_fn() -> Option<String> {
            if !#gen_typescript {
                return None;
            }
            Some(#ts_def.to_string())
        }

        /// Returns metadata about this exposed function.
        #fn_vis fn #register_fn() -> blueprint_types::ExposedFunction {
            blueprint_types::ExposedFunction {
                name: #exposed_name.to_string(),
                description: #description_expr,
                js_enabled: #gen_js,
                blueprint_enabled: #gen_blueprint,
                typescript_enabled: #gen_typescript,
                category: #category.to_string(),
                pure: #is_pure,
                typescript_def: #ts_def.to_string(),
            }
        }
    };

    output.into()
}
