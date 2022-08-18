use self::api::{ApiTable, BootstrapContext, LuaFunction, LuaReg, LuaState};
use std::collections::LinkedList;
use std::ffi::{c_void, CStr, CString};
use std::mem::{size_of, transmute};
use std::os::raw::{c_int, c_uint};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::unreachable;

pub mod api;

pub const LUAI_IS32INT: bool = (c_uint::MAX >> 30) >= 3;
pub const LUAI_MAXSTACK: c_int = if LUAI_IS32INT { 1000000 } else { 15000 };
pub const LUA_REGISTRYINDEX: c_int = -LUAI_MAXSTACK - 1000;

pub const LUA_TNIL: c_int = 0;
pub const LUA_TBOOLEAN: c_int = 1;
pub const LUA_TLIGHTUSERDATA: c_int = 2;
pub const LUA_TNUMBER: c_int = 3;
pub const LUA_TSTRING: c_int = 4;
pub const LUA_TTABLE: c_int = 5;
pub const LUA_TFUNCTION: c_int = 6;
pub const LUA_TUSERDATA: c_int = 7;
pub const LUA_TTHREAD: c_int = 8;

pub static mut API_TABLE: *const ApiTable = null();

/// A helper macro that combine `error_with_message` and `format` together.
///
/// # Examples
///
/// ```no_run
/// # let lua: *mut locenv::api::LuaState = std::ptr::null_mut();
/// # let e = "abc";
/// locenv::error!(lua, "Something went wrong: {}", e);
/// ```
#[macro_export]
macro_rules! error {
    ($lua:ident, $($arg:tt)*) => {
        $crate::error_with_message($lua, &std::format!($($arg)*))
    }
}

/// Returns the pseudo-index that represents the i-th upvalue of the running function. i must be in the range [1,256].
pub fn upvalue_index<P: Into<c_int>>(i: P) -> c_int {
    LUA_REGISTRYINDEX - i.into()
}

/// Converts the acceptable index `index` into an equivalent absolute index (that is, one that does
/// not depend on the stack size).
pub fn abs_index(lua: *mut LuaState, index: c_int) -> c_int {
    (api().lua_absindex)(lua, index)
}

/// Pops `count` elements from the stack.
pub fn pop(lua: *mut LuaState, count: c_int) {
    (api().lua_settop)(lua, -count - 1);
}

/// Pushes a copy of the element at the given index onto the stack.
pub fn push_value(lua: *mut LuaState, index: c_int) {
    (api().lua_pushvalue)(lua, index);
}

/// Pushes a nil value onto the stack.
pub fn push_nil(lua: *mut LuaState) {
    (api().lua_pushnil)(lua);
}

/// Pushes a string onto the stack. The string can contain any binary data, including
/// embedded zeros.
pub fn push_str(lua: *mut LuaState, value: &str) {
    unsafe { (api().lua_pushlstring)(lua, transmute(value.as_ptr()), value.len()) };
}

/// Pushes a new function onto the stack.
///
/// This function receives a Rust function and pushes onto the stack a Lua value of type function
/// that, when called, invokes the corresponding Rust function. The parameter `up` tells how many
/// upvalues this function will have.
pub fn push_fn(lua: *mut LuaState, value: LuaFunction, up: c_int) {
    (api().lua_pushcclosure)(lua, value, up);
}

/// Pushes a new closure onto the stack.
///
/// The closure will be owned by the [`Context`] at the specified `index`.
pub fn push_closure<T: Closure>(lua: *mut LuaState, context: c_int, value: T) {
    let context = abs_index(lua, context);

    push_value(lua, context);
    create_userdata(lua, context, value, |_, _, _| {});
    push_fn(lua, execute_closure::<T>, 2);
}

/// Creates a new empty table and pushes it onto the stack. Parameter `elements` is a hint for how many
/// elements the table will have as a sequence; parameter `fields` is a hint for how many other elements the
/// table will have. Lua may use these hints to preallocate memory for the new table. This preallocation may
/// help performance when you know in advance how many elements the table will have.
pub fn create_table(lua: *mut LuaState, elements: c_int, fields: c_int) {
    (api().lua_createtable)(lua, elements, fields);
}

/// This function creates and pushes on the stack a new full userdata, with Rust object associated
/// Lua values.
///
/// The userdata will be owned by the [`Context`] at the specified `index`.
pub fn new_userdata<T: Object>(lua: *mut LuaState, context: c_int, value: T) {
    create_userdata(lua, context, value, |lua, context, _| {
        let methods = T::methods();

        if methods.is_empty() {
            return;
        }

        create_table(lua, 0, methods.len() as _);

        for method in T::methods() {
            push_value(lua, context);
            (api().lua_pushlightuserdata)(lua, unsafe { transmute(method.function) });
            push_fn(lua, invoke_method::<T>, 2);
            set_field(lua, -2, method.name);
        }

        set_field(lua, -2, "__index");
    });
}

/// Does the equivalent to t[key] = v, where t is the value at the given `index` and v is the value
/// on the top of the stack.
///
/// This function pops the value from the stack. As in Lua, this function may trigger a metamethod
/// for the "newindex" event.
pub fn set_field(lua: *mut LuaState, index: c_int, key: &str) {
    let key = CString::new(key).unwrap();

    unsafe { (api().lua_setfield)(lua, index, key.as_ptr()) };
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
    unsafe { (api().aux_setfuncs)(lua, table.as_ptr(), upvalues) };
}

/// Returns `true` if the given `index` is not valid or if the value at this `index` is nil, and
/// `false` otherwise.
pub fn is_none_or_nil(lua: *mut LuaState, index: c_int) -> bool {
    (api().lua_type)(lua, index) <= 0
}

/// If the function argument `arg` is a string, returns this string. If this argument is absent or
/// is nil, returns [`None`]. Otherwise, raises an error.
///
/// This function uses [`to_string`] to get its result, so all conversions and caveats of that
/// function apply here.
pub fn opt_string(lua: *mut LuaState, arg: c_int) -> Option<String> {
    if is_none_or_nil(lua, arg) {
        None
    } else {
        Some(check_string(lua, arg))
    }
}

/// Checks whether the function argument `arg` is a string and returns this string.
pub fn check_string(lua: *mut LuaState, arg: c_int) -> String {
    let data = unsafe { (api().aux_checklstring)(lua, arg, null_mut()) };
    let raw = unsafe { CStr::from_ptr(data) };

    raw.to_str().unwrap().into()
}

/// Converts the Lua value at the given `index` to a string.
///
/// The Lua value must be a string or a number; otherwise, the function returns [`None`]. If the value is a number,
/// then this function also changes the actual value in the stack to a string.
pub fn to_string(lua: *mut LuaState, index: c_int) -> Option<String> {
    let value = unsafe { (api().lua_tolstring)(lua, index, null_mut()) };

    if value.is_null() {
        return None;
    }

    unsafe { Some(CStr::from_ptr(value).to_str().unwrap().into()) }
}

/// Pushes onto the stack the value t[key], where t is the value at the given `index`. As in Lua, this function may
/// trigger a metamethod for the "index" event.
///
/// Returns the type of the pushed value.
pub fn get_field(lua: *mut LuaState, index: c_int, key: &str) -> c_int {
    let key = CString::new(key).unwrap();

    unsafe { (api().lua_getfield)(lua, index, key.as_ptr()) }
}

/// Pops a table or nil from the stack and sets that value as the new metatable for the value at the
/// given `index` (nil means no metatable).
pub fn set_metatable(lua: *mut LuaState, index: c_int) {
    (api().lua_setmetatable)(lua, index);
}

/// Raises a type error for the argument `arg` of the function that called it, using a standard message; `expect` is
/// a "name" for the expected type.
pub fn type_error(lua: *mut LuaState, arg: c_int, expect: &str) -> ! {
    let expect = CString::new(expect).unwrap();

    unsafe { (api().aux_typeerror)(lua, arg, expect.as_ptr()) };
    unreachable!();
}

/// Raises an error reporting a problem with argument `arg` of the function that called it, using a standard message
/// that includes `comment` as a comment:
///
/// `bad argument #arg to 'funcname' (comment)`
pub fn argument_error(lua: *mut LuaState, arg: c_int, comment: &str) -> ! {
    let comment = CString::new(comment).unwrap();

    unsafe { (api().aux_argerror)(lua, arg, comment.as_ptr()) };
    unreachable!();
}

/// Raises a Lua error with the specified message.
pub fn error_with_message(lua: *mut LuaState, message: &str) -> ! {
    let format = CString::new("%s").unwrap();
    let message = CString::new(message).unwrap();

    unsafe { (api().aux_error)(lua, format.as_ptr(), message.as_ptr()) };
    unreachable!();
}

/// Raises a Lua error, using the value on the top of the stack as the error object.
pub fn error(lua: *mut LuaState) -> ! {
    (api().lua_error)(lua);
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
    /// # let lua: *mut locenv::api::LuaState = std::ptr::null_mut();
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

/// Represents the execution context of the current function.
pub struct Context {
    locenv: *const c_void,
    module_name: String,
    working_directory: PathBuf,
}

impl Context {
    pub unsafe fn new(bootstrap: *const BootstrapContext) -> Self {
        Self {
            locenv: (*bootstrap).locenv,
            module_name: CStr::from_ptr((*bootstrap).name).to_str().unwrap().into(),
            working_directory: CStr::from_ptr((*bootstrap).name).to_str().unwrap().into(),
        }
    }

    /// Gets a reference to the context from Lua stack at the specified index.
    ///
    /// **The returned reference is valid as long as the value at the specified index alive**.
    pub fn from_lua(lua: *mut LuaState, index: c_int) -> &'static Self {
        // Get userdata.
        let ud = (api().lua_touserdata)(lua, index);

        if ud.is_null() {
            error!(lua, "expect a userdata at #{}", index);
        }

        // Get type name.
        if (api().lua_getmetatable)(lua, index) == 0 {
            error!(lua, "expect a module context at #{}", index);
        }

        if get_field(lua, -1, "__name") != LUA_TSTRING {
            pop(lua, 2);
            error!(lua, "invalid metatable for the value at #{}", index);
        }

        let r#type = to_string(lua, -1).unwrap();

        pop(lua, 2);

        // Check if it is a Context.
        if r#type == "locenv" || r#type.contains('.') {
            error!(lua, "expect a module context at #{}", index);
        }

        // Dereference.
        let context: *mut Self = null_mut();

        unsafe { ud.copy_to_nonoverlapping(transmute(&context), size_of::<*mut Self>()) };
        unsafe { &*context }
    }

    /// Gets name of the current module.
    pub fn module_name(&self) -> &str {
        self.module_name.as_ref()
    }

    /// Gets a full path to the directory where the current Lua script is working on.
    pub fn working_directory(&self) -> &Path {
        self.working_directory.as_ref()
    }

    /// Gets a full path where to store configurations for the current module. The returned value is in the following form:
    ///
    /// `$LOCENV_DATA/config/<module>`
    pub fn configurations_path(&self) -> PathBuf {
        let name = CString::new(self.module_name.as_str()).unwrap();
        let mut size: u32 = 256;

        loop {
            let mut buffer: Vec<u8> = Vec::with_capacity(size as _);
            let result = unsafe {
                (api().module_configurations_path)(
                    self.locenv,
                    name.as_ptr(),
                    buffer.as_mut_ptr() as *mut _,
                    size,
                )
            };

            if result <= size {
                unsafe { buffer.set_len((result - 1) as _) };
                return unsafe { PathBuf::from(String::from_utf8_unchecked(buffer)) };
            }

            size *= 2;
        }
    }

    /// A finalizer for [`Context`]. This method is used by #\[loader\] attribute.
    pub extern "C" fn finalize(lua: *mut LuaState) -> c_int {
        // Get a pointer to context.
        let table = unsafe { (api().aux_checklstring)(lua, upvalue_index(1), null_mut()) };
        let ud = unsafe { (api().aux_checkudata)(lua, 1, table) };
        let raw: *mut Self = null_mut();

        unsafe { ud.copy_to_nonoverlapping(transmute(&raw), size_of::<*mut Self>()) };

        // Destroy.
        unsafe { Box::from_raw(raw) };

        0
    }

    fn get_userdata<T: UserData>(&self, lua: *mut LuaState, index: c_int) -> *mut T {
        let table = self.get_type_name::<T>();
        let table = CString::new(table).unwrap();
        let ud = unsafe { (api().aux_checkudata)(lua, index, table.as_ptr()) };
        let raw: *mut T = null_mut();

        unsafe { ud.copy_to_nonoverlapping(transmute(&raw), size_of::<*mut T>()) };

        raw
    }

    fn get_type_name<T: UserData>(&self) -> String {
        format!("{}.{}", self.module_name, T::type_name())
    }
}

fn create_userdata<T, S>(lua: *mut LuaState, context: c_int, value: T, setup: S)
where
    T: UserData,
    S: FnOnce(*mut LuaState, c_int, &T),
{
    // Get table name.
    let context = abs_index(lua, context);
    let table = Context::from_lua(lua, context).get_type_name::<T>();
    let table = CString::new(table).unwrap();

    // Push the userdata.
    let boxed = Box::into_raw(Box::new(value));
    let size = size_of::<*mut T>();
    let up = (api().lua_newuserdatauv)(lua, size, 1);

    unsafe { up.copy_from_nonoverlapping(transmute(&boxed), size) };

    // Associate the userdata with metatable.
    if unsafe { (api().aux_newmetatable)(lua, table.as_ptr()) } == 1 {
        push_value(lua, context);
        push_fn(lua, free_userdata::<T>, 1);
        set_field(lua, -2, "__gc");
        unsafe { setup(lua, context, &*boxed) };
    }

    set_metatable(lua, -2);
}

extern "C" fn execute_closure<T: Closure>(lua: *mut LuaState) -> c_int {
    let context = Context::from_lua(lua, upvalue_index(1));
    let closure = context.get_userdata::<T>(lua, upvalue_index(2));

    unsafe { (*closure).call(lua) }
}

extern "C" fn invoke_method<T: Object>(lua: *mut LuaState) -> c_int {
    let context = Context::from_lua(lua, upvalue_index(1));
    let method = (api().lua_touserdata)(lua, upvalue_index(2));
    let method: Method<T> = unsafe { transmute(method) };
    let data = context.get_userdata::<T>(lua, 1);

    unsafe { method(&mut *data, lua) }
}

extern "C" fn free_userdata<T: UserData>(lua: *mut LuaState) -> c_int {
    let context = Context::from_lua(lua, upvalue_index(1));
    unsafe { Box::from_raw(context.get_userdata::<T>(lua, 1)) };
    0
}

fn api() -> &'static ApiTable {
    unsafe { &*API_TABLE }
}
