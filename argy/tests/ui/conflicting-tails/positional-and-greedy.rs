/// Command
#[derive(argy::FromArgs)]
struct Cmd {
    #[argy(positional)]
    /// positional
    positional: Vec<String>,

    #[argy(positional, greedy)]
    /// remainder
    remainder: Vec<String>,
}

fn main() {}
