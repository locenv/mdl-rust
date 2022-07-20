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
        pub unsafe extern "C" fn bootstrap(bootstrap: *const locenv::api::BootstrapContext, api: *const locenv::api::ApiTable) -> std::os::raw::c_int {
            if locenv::API_TABLE.is_null() {
                locenv::API_TABLE = api;
            }

            let lua = (*bootstrap).lua;
            let context = locenv::Context::new(bootstrap);

            ((*api).lua_pushcclosure)(lua, #loader, 0);

            // Move context to user data.
            let raw = Box::into_raw(Box::new(context));
            let ptr = std::mem::size_of::<*mut locenv::Context>();
            let ud = ((*api).lua_newuserdatauv)(lua, ptr, 1);

            ud.copy_from_nonoverlapping(std::mem::transmute(&raw), ptr);

            // Associate the userdata with metatable.
            if ((*api).aux_newmetatable)(lua, (*bootstrap).name) == 0 {
                ((*api).lua_settop)(lua, -3); // Pop metatable and user data.

                // Push error.
                let context = Box::from_raw(raw);
                let error = std::ffi::CString::new(format!("someone already created a metatable named '{}'", context.module_name())).unwrap();
                ((*api).lua_pushstring)(lua, error.as_ptr());
                return 1;
            }

            ((*api).lua_pushstring)(lua, b"__gc\0");
            ((*api).lua_pushstring)(lua, (*bootstrap).name);
            ((*api).lua_pushcclosure)(lua, locenv::Context::finalize, 1);
            ((*api).lua_settable)(lua, -3);
            ((*api).lua_setmetatable)(lua, -2);

            2
        }
    };

    result.into()
}
