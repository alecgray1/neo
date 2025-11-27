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
