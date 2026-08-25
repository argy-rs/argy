/// Command
#[derive(argy::FromArgs)]
struct Cmd {
    #[argy(switch)]
    /// non-ascii
    привет: bool,
    #[argy(switch)]
    /// invalid character
    XMLHTTPRequest: bool,
    #[argy(switch, long = "invalid_character")]
    /// bad attr
    ok: bool,
}

fn main() {}
