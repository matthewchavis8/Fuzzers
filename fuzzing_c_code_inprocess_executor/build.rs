use cc::Build;


fn main() {
    Build::new()
        .file("./src/harness.c")
        .compile("harness.a");

}
