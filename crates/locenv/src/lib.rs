pub use self::api::LuaState;

use self::api::{ApiTable, Locenv, LuaFunction, LuaReg};
use std::collections::LinkedList;
use std::ffi::{CStr, CString};
use std::mem::{size_of, transmute};
use std::os::raw::{c_int, c_uint};
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::unreachable;

pub mod api;

pub const LUAI_IS32INT: bool = (c_uint::MAX >> 30) >= 3;
pub const LUAI_MAXSTACK: c_int = if LUAI_IS32INT { 1000000 } else { 15000 };
pub const LUA_REGISTRYINDEX: c_int = -LUAI_MAXSTACK - 1000;

pub static mut MODULE_NAME: String = String::new();
pub static mut CONTEXT: *const Locenv = null();
pub static mut WORKING_DIRECTORY: String = String::new();
pub static mut API_TABLE: *const ApiTable = null();

/// A helper macro that combine `error_with_message` and `format` together.
///
/// # Examples
///
/// ```no_run
/// # let lua: *mut locenv::LuaState = std::ptr::null_mut();
/// # let e = "abc";
/// locenv::error!(lua, "Something went wrong: {}", e);
/// ```
#[macro_export]
macro_rules! error {
    ($lua:ident, $($arg:tt)*) => {
        locenv::error_with_message($lua, &std::format!($($arg)*))
    }
}

/// Gets name of the current module.
pub fn get_module_name() -> &'static str {
    unsafe { &MODULE_NAME }
}

/// Gets a full path to the directory where the current Lua script is working on.
pub fn get_working_directory() -> &'static str {
    unsafe { &WORKING_DIRECTORY }
}

/// Gets a full path where to store configurations for this module. The returned value is in the following form:
///
/// `$LOCENV_DATA/config/<module>`
pub fn get_configurations_path() -> PathBuf {
    let name = CString::new(get_module_name()).unwrap();
    let mut size: u32 = 256;

    loop {
        let mut buffer: Vec<u8> = Vec::with_capacity(size as usize);
        let result = unsafe {
            (get_api().module_configurations_path)(
                CONTEXT,
                name.as_ptr(),
                buffer.as_mut_ptr() as *mut _,
                size,
            )
        };

        if result <= size {
            unsafe { buffer.set_len((result - 1) as usize) };

            return unsafe { PathBuf::from(String::from_utf8_unchecked(buffer)) };
        }

        size *= 2;
    }
}

/// Returns the pseudo-index that represents the i-th upvalue of the running function. i must be in the range [1,256].
pub fn get_upvalue_index<P: Into<c_int>>(i: P) -> c_int {
    LUA_REGISTRYINDEX - i.into()
}

/// Pushes a nil value onto the stack.
pub fn push_nil(lua: *mut LuaState) {
    (get_api().lua_pushnil)(lua);
}

/// Pushes a string onto the stack. The string can contain any binary data, including
/// embedded zeros.
pub fn push_str(lua: *mut LuaState, value: &str) {
    unsafe { (get_api().lua_pushlstring)(lua, transmute(value.as_ptr()), value.len()) };
}

/// Pushes a new function onto the stack. This function receives a Rust function and pushes
/// onto the stack a Lua value of type function that, when called, invokes the corresponding
/// Rust function.
pub fn push_fn(lua: *mut LuaState, value: LuaFunction) {
    (get_api().lua_pushcclosure)(lua, value, 0);
}

/// Pushes a new closure onto the stack.
pub fn push_closure<T: Closure>(lua: *mut LuaState, value: T) {
    create_userdata(lua, value, |_, _| {});
    (get_api().lua_pushcclosure)(lua, execute_closure::<T>, 1);
}

/// Creates a new empty table and pushes it onto the stack. Parameter `elements` is a hint for how many
/// elements the table will have as a sequence; parameter `fields` is a hint for how many other elements the
/// table will have. Lua may use these hints to preallocate memory for the new table. This preallocation may
/// help performance when you know in advance how many elements the table will have.
pub fn create_table(lua: *mut LuaState, elements: c_int, fields: c_int) {
    (get_api().lua_createtable)(lua, elements, fields);
}

/// This function creates and pushes on the stack a new full userdata, with Rust object associated Lua values.
pub fn new_userdata<T: Object>(lua: *mut LuaState, value: T) {
    create_userdata(lua, value, |lua, _| {
        let methods = T::methods();

        if methods.is_empty() {
            return;
        }

        push_str(lua, "__index");
        create_table(lua, 0, methods.len() as c_int);

        for method in T::methods() {
            push_str(lua, method.name);
            (get_api().lua_pushlightuserdata)(lua, unsafe { transmute(method.function) });
            (get_api().lua_pushcclosure)(lua, invoke_method::<T>, 1);
            (get_api().lua_settable)(lua, -3);
        }

        (get_api().lua_settable)(lua, -3);
    });
}

/// Registers all functions in the `entries` into the table on the top of the stack. When `upvalues` is not zero,
/// all functions are created with `upvalues` upvalues, initialized with copies of the `upvalues` values previously
/// pushed on the stack on top of the library table. These values are popped from the stack after the registration.
pub fn set_functions(lua: *mut LuaState, entries: &[FunctionEntry], upvalues: c_int) {
    // Build a table of LuaReg.
    let mut table: Vec<LuaReg> = Vec::new();
    let mut names: LinkedList<CString> = LinkedList::new();

    for e in entries {
        names.push_back(CString::new(e.name).unwrap());

        table.push(LuaReg {
            name: names.back().unwrap().as_ptr(),
            func: e.function,
        });
    }

    table.push(LuaReg {
        name: null(),
        func: None,
    });

    // Register.
    unsafe { (get_api().aux_setfuncs)(lua, table.as_ptr(), upvalues) };
}

/// Checks whether the function argument `arg` is a string and returns this string.
pub fn check_string(lua: *mut LuaState, arg: c_int) -> String {
    let data = unsafe { (get_api().aux_checklstring)(lua, arg, null_mut()) };
    let raw = unsafe { CStr::from_ptr(data) };

    raw.to_str().unwrap().into()
}

/// Raises a type error for the argument `arg` of the function that called it, using a standard message; `expect` is
/// a "name" for the expected type.
pub fn type_error(lua: *mut LuaState, arg: c_int, expect: &str) -> ! {
    let expect = CString::new(expect).unwrap();

    unsafe { (get_api().aux_typeerror)(lua, arg, expect.as_ptr()) };
    unreachable!();
}

/// Raises an error reporting a problem with argument `arg` of the function that called it, using a standard message
/// that includes `comment` as a comment:
///
/// `bad argument #arg to 'funcname' (comment)`
pub fn argument_error(lua: *mut LuaState, arg: c_int, comment: &str) -> ! {
    let comment = CString::new(comment).unwrap();

    unsafe { (get_api().aux_argerror)(lua, arg, comment.as_ptr()) };
    unreachable!();
}

/// Raises a Lua error with the specified message.
pub fn error_with_message(lua: *mut LuaState, message: &str) -> ! {
    let format = CString::new("%s").unwrap();
    let message = CString::new(message).unwrap();

    unsafe { (get_api().aux_error)(lua, format.as_ptr(), message.as_ptr()) };
    unreachable!();
}

/// Raises a Lua error, using the value on the top of the stack as the error object.
pub fn error(lua: *mut LuaState) -> ! {
    (get_api().lua_error)(lua);
    unreachable!();
}

/// A trait to allow Rust object to be able to get collected by Lua GC.
pub trait UserData: 'static {
    /// Gets a unique name for this type within this module.
    fn type_name() -> &'static str;
}

/// A trait for implement Lua closure.
pub trait Closure: UserData {
    fn call(&mut self, lua: *mut LuaState) -> c_int;
}

/// A trait for implement Lua object.
pub trait Object: UserData {
    /// Gets a set of available methods.
    fn methods() -> &'static [MethodEntry<Self>];
}

/// Represents a method of a Lua object.
pub struct MethodEntry<T: ?Sized> {
    pub name: &'static str,

    /// A pointer to function for this method.
    ///
    /// Please note that the first argument for the method is on the **second** index, not the first index.
    /// Let say the user invoke your method as the following:
    ///
    /// ```notrust
    /// v:method('abc')
    /// ```
    ///
    /// Within this function you can get 'abc' with:
    ///
    /// ```no_run
    /// # let lua: *mut locenv::LuaState = std::ptr::null_mut();
    /// locenv::check_string(lua, 2);
    /// ```
    ///
    /// Notice the index is `2`, not `1`.
    pub function: Method<T>,
}

pub type Method<T> = fn(&mut T, *mut LuaState) -> c_int;

/// Represents a function to add to a Lua table.
pub struct FunctionEntry<'name> {
    pub name: &'name str,
    pub function: Option<LuaFunction>,
}

fn create_userdata<T: UserData>(lua: *mut LuaState, value: T, setup: fn(*mut LuaState, &T)) {
    // Get table name.
    let table = get_type_name::<T>();
    let table = CString::new(table).unwrap();

    // Push the userdata.
    let boxed = Box::into_raw(Box::new(value));
    let size = size_of::<*mut T>();
    let up = (get_api().lua_newuserdatauv)(lua, size, 1);

    unsafe { up.copy_from_nonoverlapping(transmute(&boxed), size) };

    // Associate the userdata with metatable.
    if unsafe { (get_api().aux_newmetatable)(lua, table.as_ptr()) } == 1 {
        push_str(lua, "__gc");
        (get_api().lua_pushcclosure)(lua, free_userdata::<T>, 0);
        (get_api().lua_settable)(lua, -3);
        unsafe { setup(lua, &*boxed) };
    }

    (get_api().lua_setmetatable)(lua, -2);
}

extern "C" fn execute_closure<T: Closure>(lua: *mut LuaState) -> c_int {
    let closure = unsafe { get_userdata::<T>(lua, get_upvalue_index(1)) };

    unsafe { (*closure).call(lua) }
}

extern "C" fn invoke_method<T: Object>(lua: *mut LuaState) -> c_int {
    let method = (get_api().lua_touserdata)(lua, get_upvalue_index(1));
    let method: Method<T> = unsafe { transmute(method) };
    let data = unsafe { get_userdata::<T>(lua, 1) };

    unsafe { method(&mut *data, lua) }
}

extern "C" fn free_userdata<T: UserData>(lua: *mut LuaState) -> c_int {
    unsafe { Box::from_raw(get_userdata::<T>(lua, 1)) };
    0
}

unsafe fn get_userdata<T: UserData>(lua: *mut LuaState, index: c_int) -> *mut T {
    let table = get_type_name::<T>();
    let table = CString::new(table).unwrap();
    let data = (get_api().aux_checkudata)(lua, index, table.as_ptr());
    let boxed: *mut T = null_mut();

    if data.is_null() {
        let error = format!(
            "expect a value with type '{}' at #{}",
            get_type_name::<T>(),
            index
        );

        error_with_message(lua, &error);
    }

    data.copy_to_nonoverlapping(transmute(&boxed), size_of::<*mut T>());

    boxed
}

fn get_type_name<T: UserData>() -> String {
    format!("{}.userdata.{}", get_module_name(), T::type_name())
}

fn get_api() -> &'static ApiTable {
    unsafe { &*API_TABLE }
}
