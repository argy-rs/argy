/// Command
#[derive(argy::FromArgs)]
struct Cmd {
    #[argy(positional, last)]
    /// first
    first: Vec<String>,
    #[argy(positional)]
    /// second
    second: String,
}

fn main() {}
