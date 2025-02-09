mod rebuild;
use crate::rebuild::rebuild;
use rebuild::*;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, punctuated::Punctuated, token::Comma, FnArg, ItemFn, Pat};

fn guess_shared_library_filename(base_name: &str) -> String {
    if cfg!(target_os = "linux") {
        format!("lib{}.so", base_name)
    } else if cfg!(target_os = "macos") {
        format!("lib{}.dylib", base_name)
    } else if cfg!(target_os = "windows") {
        format!("{}.dll", base_name)
    } else {
        panic!("Unsupported OS");
    }
}

use proc_macro2::{Ident, Span};

fn get_argument_names(args: &Punctuated<FnArg, Comma>) -> proc_macro2::TokenStream {
    let mut arg_names = Vec::new();

    for arg in args {
        match arg {
            // Handle `self`, `&self`, or `&mut self`
            FnArg::Receiver(_) => {
                arg_names.push(Ident::new("self", Span::call_site()));
            }
            // Handle named function parameters
            FnArg::Typed(pat) => {
                if let Pat::Ident(ident) = *pat.pat.clone() {
                    arg_names.push(ident.ident);
                }
            }
        }
    }

    quote! { #(#arg_names),* }
}

fn get_argument_types(args: &Punctuated<FnArg, Comma>) -> proc_macro2::TokenStream {
    let mut arg_types = Vec::new();

    for arg in args {
        match arg {
            // Handle `self`, `&self`, or `&mut self`
            FnArg::Receiver(receiver) => {
                let self_type = if receiver.reference.is_some() {
                    if receiver.mutability.is_some() {
                        quote! { &mut Self }
                    } else {
                        quote! { &Self }
                    }
                } else {
                    quote! { Self }
                };
                arg_types.push(self_type);
            }
            // Handle regular function parameters
            FnArg::Typed(pat) => {
                arg_types.push(pat.ty.to_token_stream());
            }
        }
    }

    quote! { #(#arg_types),* }
}

#[proc_macro_attribute]
pub fn hot_reload(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);

    let fn_signature = &input_fn.sig;
    let fn_block = &input_fn.block;
    let args = &fn_signature.inputs;

    
    let arg_types = get_argument_types(args);
    
    let arg_names = get_argument_names(args);
    
    // Check if we are building the main crate or the shared library
    let is_in_main_crate = std::env::var(KAUMA_ENV_VAR).is_err();

    // If we're in the main crate, generate the code to load the shared library
    if is_in_main_crate {
        // Run a first rebuild at compile time. Not sure if this is always ok.
        let _ = rebuild();

        let cargo_target_dir = cargo_target_dir();
        let cargo_target_dir = cargo_target_dir.to_str().unwrap_or_else(|| panic!("Invalid path"));

        let shared_lib = guess_shared_library_filename(KAUMA_SHARED_LIB_NAME);

        let expanded = quote! {
            pub #fn_signature {
                // Try to load the shared library
                let lib_path = std::path::Path::new(#cargo_target_dir)
                    .join(#KAUMA_HOT_BUILD_DIR)
                    .join("target")
                    .join("debug")
                    .join(#shared_lib);

                let lib = unsafe { libloading::Library::new(lib_path.clone()) };
                let lib = match lib {
                    Ok(lib) => lib,
                    Err(_) => {
                        // In case of failure, run the regular function.
                        eprintln!("Hot reload failed: Couldn't find the shared library at {:?}.", lib_path);
                        return #fn_block;
                    }
                };

                // Try to load the function symbol
                let func: Result<libloading::Symbol<unsafe extern "C" fn(#arg_types)>, _> = unsafe {
                    lib.get(b"do_stuff")
                };

                let func = match func {
                    Ok(func) => func,
                    Err(_) => {
                        eprintln!("Hot reload failed: Couldn't find the function in the shared library.");
                        return #fn_block;
                    }
                };

                // Run the loaded function
                unsafe { func(#arg_names); }
            }
        };
        expanded.into()
    } else {
        // When building the shared object, just add #[no_mangle]
        let expanded = quote! {
            #[no_mangle]
            #input_fn
        };
        expanded.into()
    }
}
