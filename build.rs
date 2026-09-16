pub fn main(){ 
    csbindgen::Builder::default()
        .input_extern_file("src/lib.rs")
        .csharp_dll_name("eegy")
        .generate_csharp_file("eegy-unity/Assets/eegy/Native/NativeMethods.g.cs")
        .unwrap();
}