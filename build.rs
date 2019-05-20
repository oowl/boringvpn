use cc;

fn main() {
    cc::Build::new()
        .file("src/c/tuntap.c")
        .compile("tuntap.a");
}