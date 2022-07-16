use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn loader(_: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let loader = &input.sig.ident;
    let result = quote! {
        #input

        #[no_mangle]
        pub unsafe extern "C" fn bootstrap(context: *const locenv::api::BootstrapContext, api: *const locenv::api::ApiTable) -> std::os::raw::c_int {
            locenv::MODULE_NAME = std::ffi::CStr::from_ptr((*context).name).to_str().unwrap().into();
            locenv::CONTEXT = (*context).locenv;
            locenv::WORKING_DIRECTORY = std::ffi::CStr::from_ptr((*context).working_directory).to_str().unwrap().into();
            locenv::API_TABLE = api;

            ((*api).lua_pushcclosure)((*context).lua, #loader, 0);
            ((*api).lua_pushstring)((*context).lua, (*context).name);

            2
        }
    };

    result.into()
}
