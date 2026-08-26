/// Command
#[derive(argy::FromArgs)]
struct Cmd {
    #[argy(positional, env = "POS_ENV")]
    /// positional
    positional: String,

    #[argy(option, env = "DUP_ONE", env = "DUP_TWO")]
    /// option
    opt: Option<String>,

    #[argy(option, env = "VEC_ENV")]
    /// repeating
    repeat: Vec<String>,
}

fn main() {}
