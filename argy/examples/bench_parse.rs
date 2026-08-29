// Representative argy parse workload: a realistic CLI with options, a
// subcommand, switches and positionals, parsed in a tight loop. Used by the
// optimization coordinator as the runtime-speed harness for A/B testing.
use argy::FromArgs;
use std::time::Instant;

#[derive(FromArgs, PartialEq, Debug)]
#[argy(description = "bench command", example = "bench --height 5 jump --power 7 --loud moon")]
struct CliArgs {
    /// how high to go
    #[argy(option)]
    height: usize,
    /// an optional nickname for the pilot
    #[argy(option)]
    pilot_nickname: Option<String>,
    /// command to execute
    #[argy(subcommand)]
    command: Command,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argy(subcommand)]
enum Command {
    Jump(JumpCmd),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argy(subcommand, name = "jump", short = 'j')]
/// jump subcommand
struct JumpCmd {
    /// power level
    #[argy(option)]
    power: usize,
    /// whether to be loud
    #[argy(switch)]
    loud: bool,
    /// target positional
    #[argy(positional)]
    target: Option<String>,
}

fn main() {
    let argv =
        ["--height", "5", "--pilot-nickname", "Wes", "jump", "--power", "7", "--loud", "moon"];
    let n: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(200_000);

    // Warm up.
    let _: CliArgs = CliArgs::from_args(&["bench"], &argv).unwrap();

    let start = Instant::now();
    let mut acc = 0usize;
    for _ in 0..n {
        let parsed: CliArgs = CliArgs::from_args(&["bench"], &argv).unwrap();
        acc = acc.wrapping_add(parsed.height);
    }
    let elapsed = start.elapsed();
    let ns = elapsed.as_nanos() / n as u128;
    println!("parsed {n} iters in {elapsed:?} ({ns} ns/iter) acc={acc}");
}
