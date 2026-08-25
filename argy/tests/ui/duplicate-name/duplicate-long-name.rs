/// Command
#[derive(argy::FromArgs)]
struct Cmd {
    /// foo1
    #[argy(option, long = "foo")]
    foo1: u32,

    /// foo2
    #[argy(option, long = "foo")]
    foo2: u32,

    /// bar1
    #[argy(option, long = "bar")]
    bar1: u32,

    /// bar2
    #[argy(option, long = "bar")]
    bar2: u32,
}

fn main() {}
