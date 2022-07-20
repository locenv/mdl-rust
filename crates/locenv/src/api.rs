use std::ffi::c_void;
use std::os::raw::{c_char, c_double, c_int, c_longlong, c_ulonglong};

pub type LuaFunction = extern "C" fn(*mut LuaState) -> c_int;
pub type LuaContinuation = unsafe extern "C" fn(*mut LuaState, c_int, isize) -> c_int;
pub type LuaReader = unsafe extern "C" fn(*mut LuaState, *mut c_void, *mut usize) -> *const c_char;
pub type LuaWriter =
    unsafe extern "C" fn(*mut LuaState, *const c_void, usize, *mut c_void) -> c_int;
pub type LuaAlloc = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, usize) -> *mut c_void;

#[repr(C)]
pub struct BootstrapContext {
    pub revision: u32,
    pub name: *const c_char,
    pub locenv: *const c_void,
    pub lua: *mut LuaState,
    pub working_directory: *const c_char,
}

#[repr(C)]
pub struct LuaState {
    private: [u8; 0],
}

#[repr(C)]
pub struct LuaReg {
    pub name: *const c_char,
    pub func: Option<LuaFunction>,
}

#[repr(C)]
pub struct ApiTable {
    pub revision: u32,

    pub lua_pushboolean: unsafe extern "C" fn(*mut LuaState, c_int),
    pub lua_pushcclosure: extern "C" fn(*mut LuaState, LuaFunction, c_int),
    pub lua_pushfstring: unsafe extern "C" fn(*mut LuaState, *const c_char, ...) -> *const c_char,
    pub lua_pushinteger: unsafe extern "C" fn(*mut LuaState, c_longlong),
    pub lua_pushlightuserdata: extern "C" fn(*mut LuaState, *mut u8),
    pub lua_pushlstring: unsafe extern "C" fn(*mut LuaState, *const c_char, usize) -> *const c_char,
    pub lua_pushnil: extern "C" fn(*mut LuaState),
    pub lua_pushnumber: unsafe extern "C" fn(*mut LuaState, c_double),
    pub lua_pushstring: unsafe extern "C" fn(*mut LuaState, *const c_char) -> *const c_char,
    pub lua_pushthread: unsafe extern "C" fn(*mut LuaState) -> c_int,
    pub lua_pushvalue: extern "C" fn(*mut LuaState, c_int),
    pub lua_pushvfstring:
        unsafe extern "C" fn(*mut LuaState, *const c_char, *mut c_void) -> *const c_char,
    pub lua_createtable: extern "C" fn(*mut LuaState, c_int, c_int),
    pub lua_newuserdatauv: extern "C" fn(*mut LuaState, usize, c_int) -> *mut u8,

    pub lua_settable: extern "C" fn(*mut LuaState, c_int),
    pub lua_rawset: unsafe extern "C" fn(*mut LuaState, c_int),
    pub lua_seti: unsafe extern "C" fn(*mut LuaState, c_int, c_longlong),
    pub lua_rawseti: unsafe extern "C" fn(*mut LuaState, c_int, c_longlong),
    pub lua_setfield: unsafe extern "C" fn(*mut LuaState, c_int, *const c_char),
    pub lua_rawsetp: unsafe extern "C" fn(*mut LuaState, c_int, *const c_void),
    pub lua_setmetatable: extern "C" fn(*mut LuaState, c_int) -> c_int,
    pub lua_setiuservalue: unsafe extern "C" fn(*mut LuaState, c_int, c_int) -> c_int,

    pub lua_iscfunction: unsafe extern "C" fn(*mut LuaState, c_int) -> c_int,
    pub lua_isinteger: unsafe extern "C" fn(*mut LuaState, c_int) -> c_int,
    pub lua_isnumber: unsafe extern "C" fn(*mut LuaState, c_int) -> c_int,
    pub lua_isstring: unsafe extern "C" fn(*mut LuaState, c_int) -> c_int,
    pub lua_isuserdata: unsafe extern "C" fn(*mut LuaState, c_int) -> c_int,
    pub lua_type: unsafe extern "C" fn(*mut LuaState, c_int) -> c_int,
    pub lua_typename: unsafe extern "C" fn(*mut LuaState, c_int) -> *const c_char,
    pub lua_getmetatable: extern "C" fn(*mut LuaState, c_int) -> c_int,

    pub lua_toboolean: unsafe extern "C" fn(*mut LuaState, c_int) -> c_int,
    pub lua_tocfunction: unsafe extern "C" fn(*mut LuaState, c_int) -> LuaFunction,
    pub lua_tointegerx: unsafe extern "C" fn(*mut LuaState, c_int, *mut c_int) -> c_longlong,
    pub lua_tolstring: unsafe extern "C" fn(*mut LuaState, c_int, *mut usize) -> *const c_char,
    pub lua_tonumberx: unsafe extern "C" fn(*mut LuaState, c_int, *mut c_int) -> c_double,
    pub lua_topointer: unsafe extern "C" fn(*mut LuaState, c_int) -> *const c_void,
    pub lua_tothread: unsafe extern "C" fn(*mut LuaState, c_int) -> *mut LuaState,
    pub lua_touserdata: extern "C" fn(*mut LuaState, c_int) -> *mut u8,

    pub lua_geti: unsafe extern "C" fn(*mut LuaState, c_int, c_longlong) -> c_int,
    pub lua_rawgeti: unsafe extern "C" fn(*mut LuaState, c_int, c_longlong) -> c_int,
    pub lua_gettable: unsafe extern "C" fn(*mut LuaState, c_int) -> c_int,
    pub lua_rawget: unsafe extern "C" fn(*mut LuaState, c_int) -> c_int,
    pub lua_getfield: unsafe extern "C" fn(*mut LuaState, c_int, *const c_char) -> c_int,
    pub lua_rawgetp: unsafe extern "C" fn(*mut LuaState, c_int, *const c_void) -> c_int,
    pub lua_next: unsafe extern "C" fn(*mut LuaState, c_int) -> c_int,
    pub lua_getiuservalue: unsafe extern "C" fn(*mut LuaState, c_int, c_int) -> c_int,

    pub lua_getglobal: unsafe extern "C" fn(*mut LuaState, *const c_char) -> c_int,
    pub lua_setglobal: unsafe extern "C" fn(*mut LuaState, *const c_char),

    pub lua_gettop: unsafe extern "C" fn(*mut LuaState) -> c_int,
    pub lua_settop: extern "C" fn(*mut LuaState, c_int),

    pub lua_callk: unsafe extern "C" fn(*mut LuaState, c_int, c_int, isize, LuaContinuation),
    pub lua_pcallk:
        unsafe extern "C" fn(*mut LuaState, c_int, c_int, c_int, isize, LuaContinuation) -> c_int,
    pub lua_error: extern "C" fn(*mut LuaState) -> c_int,
    pub lua_warning: unsafe extern "C" fn(*mut LuaState, *const c_char, c_int),

    pub lua_checkstack: unsafe extern "C" fn(*mut LuaState, c_int) -> c_int,
    pub lua_absindex: extern "C" fn(*mut LuaState, c_int) -> c_int,
    pub lua_copy: unsafe extern "C" fn(*mut LuaState, c_int, c_int),
    pub lua_rotate: unsafe extern "C" fn(*mut LuaState, c_int, c_int),

    pub lua_len: unsafe extern "C" fn(*mut LuaState, c_int),
    pub lua_rawlen: unsafe extern "C" fn(*mut LuaState, c_int) -> c_ulonglong,
    pub lua_compare: unsafe extern "C" fn(*mut LuaState, c_int, c_int, c_int) -> c_int,
    pub lua_rawequal: unsafe extern "C" fn(*mut LuaState, c_int, c_int) -> c_int,

    pub lua_arith: unsafe extern "C" fn(*mut LuaState, c_int),
    pub lua_concat: unsafe extern "C" fn(*mut LuaState, c_int),

    pub lua_load: unsafe extern "C" fn(
        *mut LuaState,
        LuaReader,
        *mut c_void,
        *const c_char,
        *const c_char,
    ) -> c_int,
    pub lua_dump: unsafe extern "C" fn(*mut LuaState, LuaWriter, *mut c_void, c_int) -> c_int,

    pub lua_toclose: unsafe extern "C" fn(*mut LuaState, c_int),
    pub lua_closeslot: unsafe extern "C" fn(*mut LuaState, c_int),

    pub lua_stringtonumber: unsafe extern "C" fn(*mut LuaState, *const c_char) -> usize,
    pub lua_getallocf: unsafe extern "C" fn(*mut LuaState, *mut *mut c_void) -> LuaAlloc,
    pub lua_gc: unsafe extern "C" fn(*mut LuaState, c_int, ...) -> c_int,
    pub lua_version: unsafe extern "C" fn(*mut LuaState) -> c_double,

    pub aux_checkany: unsafe extern "C" fn(*mut LuaState, c_int),
    pub aux_checkinteger: unsafe extern "C" fn(*mut LuaState, c_int) -> c_longlong,
    pub aux_checklstring: unsafe extern "C" fn(*mut LuaState, c_int, *mut usize) -> *const c_char,
    pub aux_checknumber: unsafe extern "C" fn(*mut LuaState, c_int) -> c_double,
    pub aux_checkoption:
        unsafe extern "C" fn(*mut LuaState, c_int, *const c_char, *const *const c_char) -> c_int,
    pub aux_checkudata: unsafe extern "C" fn(*mut LuaState, c_int, *const c_char) -> *mut u8,
    pub aux_testudata: unsafe extern "C" fn(*mut LuaState, c_int, *const c_char) -> *mut c_void,
    pub aux_checktype: unsafe extern "C" fn(*mut LuaState, c_int, c_int),
    pub aux_typeerror: unsafe extern "C" fn(*mut LuaState, c_int, *const c_char) -> c_int,
    pub aux_argerror: unsafe extern "C" fn(*mut LuaState, c_int, *const c_char) -> c_int,

    pub aux_optinteger: unsafe extern "C" fn(*mut LuaState, c_int, c_longlong) -> c_longlong,
    pub aux_optlstring:
        unsafe extern "C" fn(*mut LuaState, c_int, *const c_char, *mut usize) -> *const c_char,
    pub aux_optnumber: unsafe extern "C" fn(*mut LuaState, c_int, c_double) -> c_double,

    pub aux_error: unsafe extern "C" fn(*mut LuaState, *const c_char, ...) -> c_int,
    pub aux_checkstack: unsafe extern "C" fn(*mut LuaState, c_int, *const c_char),
    pub aux_tolstring: unsafe extern "C" fn(*mut LuaState, c_int, *mut usize) -> *const c_char,

    pub aux_len: unsafe extern "C" fn(*mut LuaState, c_int) -> c_longlong,
    pub aux_getsubtable: unsafe extern "C" fn(*mut LuaState, c_int, *const c_char) -> c_int,
    pub aux_ref: unsafe extern "C" fn(*mut LuaState, c_int) -> c_int,
    pub aux_unref: unsafe extern "C" fn(*mut LuaState, c_int, c_int),

    pub aux_newmetatable: unsafe extern "C" fn(*mut LuaState, *const c_char) -> c_int,
    pub aux_setmetatable: unsafe extern "C" fn(*mut LuaState, *const c_char),
    pub aux_callmeta: unsafe extern "C" fn(*mut LuaState, c_int, *const c_char) -> c_int,
    pub aux_getmetafield: unsafe extern "C" fn(*mut LuaState, c_int, *const c_char) -> c_int,

    pub aux_loadstring: unsafe extern "C" fn(*mut LuaState, *const c_char) -> c_int,
    pub aux_loadfilex: unsafe extern "C" fn(*mut LuaState, *const c_char, *const c_char) -> c_int,
    pub aux_loadbufferx: unsafe extern "C" fn(
        *mut LuaState,
        *const c_char,
        usize,
        *const c_char,
        *const c_char,
    ) -> c_int,

    pub aux_setfuncs: unsafe extern "C" fn(*mut LuaState, *const LuaReg, c_int),
    pub aux_where: unsafe extern "C" fn(*mut LuaState, c_int),
    pub aux_traceback: unsafe extern "C" fn(*mut LuaState, *mut LuaState, *const c_char, c_int),
    pub aux_gsub: unsafe extern "C" fn(
        *mut LuaState,
        *const c_char,
        *const c_char,
        *const c_char,
    ) -> *const c_char,
    pub aux_execresult: unsafe extern "C" fn(*mut LuaState, c_int) -> c_int,
    pub aux_fileresult: unsafe extern "C" fn(*mut LuaState, c_int, *const c_char) -> c_int,
    pub module_configurations_path:
        unsafe extern "C" fn(*const c_void, *const c_char, *mut c_char, u32) -> u32,
}
