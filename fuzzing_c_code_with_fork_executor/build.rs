extern crate cc;

fn main() {
    cc::Build::new()
        .cpp(true)
        .file("src/harness.cpp")
        .compile("harness.a");


}
