use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Specify that the function is a module loader.
///
/// See https://www.lua.org/manual/5.4/manual.html#6.3 for more information.
///
/// # Examples
///
/// ```no_run
/// use locenv::api::LuaState;
/// use locenv_macros::loader;
/// use std::os::raw::c_int;
///
/// #[loader]
/// extern "C" fn loader(lua: *mut LuaState) -> c_int {
///     0
/// }
/// ```
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

            locenv::push_fn(lua, #loader, 0);

            // Move context to user data.
            let raw = Box::into_raw(Box::new(context));
            let ptr = std::mem::size_of::<*mut locenv::Context>();
            let ud = ((*api).lua_newuserdatauv)(lua, ptr, 1);

            ud.copy_from_nonoverlapping(std::mem::transmute(&raw), ptr);

            // Associate the userdata with metatable.
            if ((*api).aux_newmetatable)(lua, (*bootstrap).name) == 0 {
                let context = Box::from_raw(raw);

                locenv::pop(lua, 3); // Pop metatable + user data + loader.
                locenv::push_str(lua, &format!("someone already created a metatable named '{}'", context.module_name()));

                return 1;
            }

            ((*api).lua_pushstring)(lua, b"__gc\0".as_ptr() as *const _);
            ((*api).lua_pushstring)(lua, (*bootstrap).name);
            ((*api).lua_pushcclosure)(lua, locenv::Context::finalize, 1);
            ((*api).lua_settable)(lua, -3);
            ((*api).lua_setmetatable)(lua, -2);

            2
        }
    };

    result.into()
}
