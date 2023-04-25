use argh::FromArgs;

#[derive(FromArgs)]
/// A simple pomodoro timer
struct Args {
    #[argh(subcommand)]
    subcommand: SubCommands,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum SubCommands {
    SubCommandOne(Start),
    SubCommandTwo(Pause),
    SubCommandThree(Stop),
    SubCommandFour(Status),
}

#[derive(FromArgs)]
/// Start a new timer
#[argh(subcommand, name = "start")]
struct Start {}

#[derive(FromArgs)]
/// Pause the currently running timer
#[argh(subcommand, name = "pause")]
struct Pause {}

#[derive(FromArgs)]
/// Stop the currently running timer
#[argh(subcommand, name = "stop")]
struct Stop {}

#[derive(FromArgs)]
/// Get the status of the currently running timer
#[argh(subcommand, name = "status")]
struct Status {}

/**
* `main` parses the command line arguments (start, pause, stop, status) and runs other functions
* accordingly
*/
fn main() {
    let args: Args = argh::from_env();
}

/**
* `start` starts the timer with length of time specified by the user with the --duration flag
* (default 25 minutes). It also listens for incoming messages on the /tmp/pomo socket. If it gets a
* `status` message, it will answer with the time remaining. If it gets a `pause` message, it will pause the current timer. If it gets a `stop` message, it will stop the current timer and exit.
*/
fn start() {}

/**
* `pause` pauses the timer
*/
fn pause() {}

/**
* `stop` stops the timer
*/
fn stop() {}

/**
* `status` connects to the /tmp/pomo socket, sends a `status` message and prints the answer.
*/
fn status() {}
