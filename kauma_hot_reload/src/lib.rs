mod rebuild;
use crate::rebuild::rebuild;
use rebuild::*;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn hot_reload(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let fn_block = &input_fn.block;

    // Check if we are in the main crate or the shared object crate
    let is_in_main_crate = std::env::var(KAUMA_ENV_VAR).is_err();

    // If we're in the main crate, generate the code to load the shared library
    if is_in_main_crate {
        // Run a rebuild at compile time. This sounds like it could mess something up.
        let _ = rebuild();

        let cargo_target_dir = cargo_target_dir();
        let cargo_target_dir = cargo_target_dir.to_str();

        let expanded = quote! {
            pub fn #fn_name(state: &mut State) {
                // Try to load the shared library
                let lib_path = std::path::Path::new(#cargo_target_dir)
                    .join(#KAUMA_BUILD_DIR)
                    .join("target")
                    .join("debug")
                    .join("libhot_test2.so");

                let lib = unsafe { libloading::Library::new(lib_path.clone()) };

                let lib = match lib {
                    Ok(lib) => lib,
                    Err(_) => {
                        // In case of failure, run the regular function.
                        eprintln!("Hot reload failed: Couldn't find the .so file at {:?}.", lib_path);
                        return #fn_block;
                    }
                };

                // Try to load the function symbol
                let func: Result<libloading::Symbol<unsafe extern "C" fn(&mut State)>, _> = unsafe {
                    lib.get(b"do_stuff")
                };

                let func = match func {
                    Ok(func) => func,
                    Err(_) => {
                        eprintln!("Hot reload failed: Couldn't find the function in the .so file.");
                        return #fn_block;
                    }
                };

                // run the loaded function
                unsafe { func(state); }
            }
        };
        expanded.into()
    } else {
        // In the .so crate, just add #[no_mangle]
        let expanded = quote! {
            #[no_mangle]
            pub fn #fn_name(state: &mut State) {
                #fn_block
            }
        };
        expanded.into()
    }
}
