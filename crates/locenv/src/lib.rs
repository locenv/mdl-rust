use self::api::{ApiTable, LuaFunction, LuaReg, LuaState};
use std::collections::LinkedList;
use std::ffi::CString;
use std::mem::{size_of, transmute};
use std::os::raw::{c_int, c_uint};
use std::ptr::{null, null_mut};

pub mod api;

pub const LUAI_IS32INT: bool = (c_uint::MAX >> 30) >= 3;
pub const LUAI_MAXSTACK: c_int = if LUAI_IS32INT { 1000000 } else { 15000 };
pub const LUA_REGISTRYINDEX: c_int = -LUAI_MAXSTACK - 1000;

pub static mut MODULE_NAME: String = String::new();
pub static mut API_TABLE: *const ApiTable = null();

/// Gets name of the current module.
pub fn get_module_name() -> &'static str {
    unsafe { &MODULE_NAME }
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
    new_userdata(lua, value);
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
pub fn new_userdata<T: UserData>(lua: *mut LuaState, value: T) {
    // Get table name.
    let table = format!("{}.userdata.{}", get_module_name(), value.type_name());
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
    }

    (get_api().lua_setmetatable)(lua, -2);
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

/// A trait to allow Rust object to be able to get collected by Lua GC.
pub trait UserData {
    /// Gets a unique name for this type within this module.
    fn type_name(&self) -> &'static str;
}

/// A trait for implement Lua closure.
pub trait Closure: UserData {
    fn call(&mut self, lua: *mut LuaState) -> c_int;
}

pub struct FunctionEntry<'name> {
    pub name: &'name str,
    pub function: Option<LuaFunction>,
}

extern "C" fn execute_closure<T: Closure>(lua: *mut LuaState) -> c_int {
    let closure = unsafe { get_userdata::<T>(lua, get_upvalue_index(1)) };

    unsafe { (*closure).call(lua) }
}

extern "C" fn free_userdata<T: UserData>(lua: *mut LuaState) -> c_int {
    unsafe { Box::from_raw(get_userdata::<T>(lua, 1)) };
    0
}

unsafe fn get_userdata<T>(lua: *mut LuaState, index: c_int) -> *mut T {
    let data = (get_api().lua_touserdata)(lua, index);
    let boxed: *mut T = null_mut();

    data.copy_to_nonoverlapping(transmute(&boxed), size_of::<*mut T>());

    boxed
}

fn get_api() -> &'static ApiTable {
    unsafe { &*API_TABLE }
}
