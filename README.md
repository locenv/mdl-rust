# Module Development Library for Rust

This is a Rust library to aid on module development for locenv.

| Crate         | Version                                                                                               |
| ------------- | ----------------------------------------------------------------------------------------------------- |
| locenv        | [![Crates.io](https://img.shields.io/crates/v/locenv)](https://crates.io/crates/locenv)               |
| locenv-macros | [![Crates.io](https://img.shields.io/crates/v/locenv-macros)](https://crates.io/crates/locenv-macros) |

## Develop a locenv module with Rust

First create a new public repository on GitHub. Currently locenv only support installing a module from public repository on GitHub. Then clone the repository to your computer and change a directory to it. Initialize a new library project e.g.:

```sh
cargo init --lib
```

You need to add the following crates to your project:

- [locenv](https://crates.io/crates/locenv) contains safe wrapper around locenv APIs.
- [locenv-macros](https://crates.io/crates/locenv-macros) contains useful macros to build the module.

locenv required a module to be a dynamic library. Add `crate-type = ["cdylib"]` to the section `lib` inside `Cargo.toml`. e.g.:

```toml
[package]
name = "yourmodule"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
locenv = "0.1"
locenv-macros = "0.1"
```

Please note that your module might be loaded by multiple Lua VMs so take this into consideration when working with any global states.

### Sample module

```rust
// src/lib.rs
use locenv::api::LuaState;
use locenv::{Context, FunctionEntry, upvalue_index};
use locenv_macros::loader;
use std::os::raw::c_int;

const MODULE_FUNCTIONS: [FunctionEntry; 1] = [FunctionEntry {
    name: "myfunction",
    function: Some(myfunction),
}];

extern "C" fn myfunction(lua: *mut LuaState) -> c_int {
    // We can access the context here because we make it as the upvalue for this function in the loader.
    let context = Context::from_lua(lua, upvalue_index(1));

    0
}

#[loader]
extern "C" fn loader(lua: *mut LuaState) -> c_int {
    // More information about 'loader': https://www.lua.org/manual/5.4/manual.html#6.3
    // The loader data is locenv::Context.
    locenv::create_table(lua, 0, MODULE_FUNCTIONS.len() as _);
    locenv::push_value(lua, 2); // Push a loader data as upvalue for all functions in MODULE_FUNCTIONS.
    locenv::set_functions(lua, &MODULE_FUNCTIONS, 1);

    // Return a function table that we just created on above.
    1
}
```

We recommend the Lua official [manual](https://www.lua.org/manual/5.4/manual.html#4) for a quick reference. For more detailed we recommend this online [book](https://www.lua.org/pil/24.html). Please note that locenv does not support Lua coroutine due to it does not play well with Rust.

### Create module definition

```yaml
# locenv-module.yml
name: yourmodule
```

Fields other than `name` will be automatically populate for you if you use GitHub Actions to publish your release as in the next section.

### Setup GitHub Actions to publish the module

```yaml
# .github/workflows/cd.yml
name: CD
on:
  push:
    tags:
    - '*'
jobs:
  build:
    name: Build
    strategy:
      matrix:
        os: [ubuntu-20.04, macos-11, windows-2022]
        include:
        - os: ubuntu-20.04
          binary: target/release/libyourmodule.so
          artifact: amd64-linux
        - os: macos-11
          binary: target/release/libyourmodule.dylib
          artifact: amd64-darwin
        - os: windows-2022
          binary: target/release/yourmodule.dll
          artifact: amd64-win32
    runs-on: ${{ matrix.os }}
    steps:
    - name: Checkout source
      uses: actions/checkout@v3
    - name: Build
      run: cargo build -r
    - name: Upload module binary
      uses: actions/upload-artifact@v3
      with:
        name: ${{ matrix.artifact }}
        path: ${{ matrix.binary }}
  release:
    name: Release
    runs-on: ubuntu-20.04
    permissions:
      contents: write
    needs: build
    steps:
    - name: Checkout source
      uses: actions/checkout@v3
    - name: Download Linux binary
      uses: actions/download-artifact@v3
      with:
        name: amd64-linux
        path: amd64-linux
    - name: Download macOS binary
      uses: actions/download-artifact@v3
      with:
        name: amd64-darwin
        path: amd64-darwin
    - name: Download Windows binary
      uses: actions/download-artifact@v3
      with:
        name: amd64-win32
        path: amd64-win32
    - name: Prepare package content
      run: |
        mkdir -pv package
        mv -v amd64-linux/libyourmodule.so package/amd64-linux.so
        mv -v amd64-darwin/libyourmodule.dylib package/amd64-darwin.dylib
        mv -v amd64-win32/yourmodule.dll package/amd64-win32.dll
    - name: Transform module definition
      run: |
        require 'yaml'

        mod = YAML.load_file('locenv-module.yml')
        mod['version'] = Integer(ENV['GITHUB_REF_NAME'])
        mod['program'] = {
          'linux' => {
            'amd64' => 'amd64-linux.so'
          },
          'darwin' => {
            'amd64' => 'amd64-darwin.dylib'
          },
          'win32' => {
            'amd64' => 'amd64-win32.dll'
          }
        }

        File.open('package/locenv-module.yml', 'w') { |f| f.write mod.to_yaml.gsub("---\n", '') }
      shell: ruby {0}
    - name: Create package
      run: zip -r ../package.zip *
      working-directory: package
    - name: Create release
      uses: softprops/action-gh-release@v1
      with:
        files: package.zip
```

Each time you push a new tag, which tag name required to be a non-negative number (e.g. 0, 1, 2 and so on); it will create a release for you automatically.

### Install your module

```sh
locenv mod install github:user/repository
```

### Using your module

Here is the example how to use your module from build script:

```yaml
# locenv-service.yml
linux:
  build: |
    local foo = require 'yourmodule'

    foo.myfunction()
```

## License

MIT
