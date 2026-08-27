/// Command
#[derive(argy::FromArgs)]
struct Cmd {
    #[argy(option, last)]
    /// option
    option: String,
}

fn main() {}
