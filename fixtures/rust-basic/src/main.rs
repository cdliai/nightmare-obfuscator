fn main() {
    let report = rust_basic::Report::new("demo", "abc");
    println!("{} {}", report.title, report.checksum());
}
