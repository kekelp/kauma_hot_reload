use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn hot_reload(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let fn_block = &input_fn.block;

    // Check if we are in the main crate or the shared object crate
    let is_in_main_crate = std::env::var("HOT_RELOAD_BUILD").is_err();

    // If we're in the main crate, generate the code to load the shared library
    if is_in_main_crate {
        let expanded = quote! {
            pub fn #fn_name(state: &mut State) {
                // Try to load the shared library
                let lib = unsafe { libloading::Library::new("hot_stuff/target/debug/libhot_test2.so") };
                let lib = match lib {
                    Ok(lib) => lib,
                    Err(_) => {
                        // In case of failure, run the regular function.
                        eprintln!("Hot reload failed: Couldn't find the .so file. Is the hot reload server running?.");
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
                        eprintln!("Hot reload failed: Couldn't find the function in the .so file. Is the hot reload server running?.");
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
            pub extern "C" fn #fn_name(state: &mut State) {
                #fn_block
            }
        };
        expanded.into()
    }
}
