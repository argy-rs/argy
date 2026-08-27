/// Command
#[derive(argy::FromArgs)]
struct Cmd {
    #[argy(positional, last)]
    /// single
    single: String,
}

fn main() {}
