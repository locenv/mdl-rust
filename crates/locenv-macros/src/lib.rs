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
        pub extern "C" fn bootstrap(lua: *mut locenv::api::LuaState, name: *const std::os::raw::c_char, api: *const locenv::api::ApiTable) -> std::os::raw::c_int {
            unsafe { locenv::MODULE_NAME = std::ffi::CStr::from_ptr(name).to_str().unwrap().into() };
            unsafe { locenv::API_TABLE = api };
            unsafe { ((*api).lua_pushcclosure)(lua, #loader, 0) };
            unsafe { ((*api).lua_pushstring)(lua, name) };
            2
        }
    };

    result.into()
}
