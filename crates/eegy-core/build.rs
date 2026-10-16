// TODO: 将来 Unity 連携を本格実装する際は、FFI 処理とこの build.rs を `crates/eegy-ffi` クレートへ分離する。
pub fn main() {
    csbindgen::Builder::default()
        .input_extern_file("src/lib.rs")
        .csharp_dll_name("eegy")
        .csharp_namespace("Eegy.Native")
        .csharp_class_accessibility("internal")
        .generate_csharp_file("../../eegy-unity/Assets/Eegy/Native/NativeMethods.g.cs")
        .unwrap();
}