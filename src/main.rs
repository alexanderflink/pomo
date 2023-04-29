use argh::FromArgs;
use std::io::prelude::*;
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::thread;
use std::time::{Duration, Instant};

#[derive(FromArgs)]
/// A simple pomodoro timer
struct Args {
    #[argh(subcommand)]
    subcommand: SubCommands,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum SubCommands {
    Start(Start),
    Pause(Pause),
    Stop(Stop),
    Status(Status),
}

#[derive(FromArgs)]
/// Start a new timer
#[argh(subcommand, name = "start")]
struct Start {
    #[argh(option, short = 'd', default = "25")]
    /// length of timer in minutes
    duration: u64,
}

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

    match args.subcommand {
        SubCommands::Start(start_arg) => {
            let duration = Duration::new(start_arg.duration * 60, 0);

            start(duration);
        }
        SubCommands::Pause(_) => pause(),
        SubCommands::Stop(_) => stop(),
        SubCommands::Status(_) => status(),
    };
}

/**
* `start` starts the timer with length of time specified by the user with the --duration flag
* (default 25 minutes). It also listens for incoming messages on the /tmp/pomo socket. If it gets a
* `status` message, it will answer with the time remaining. If it gets a `pause` message, it will pause the current timer. If it gets a `stop` message, it will stop the current timer and exit.
*/
fn start(duration: Duration) {
    // remove socket if it exists
    std::fs::remove_file("/tmp/pomo").unwrap_or(());

    println!("Starting timer for {} minutes", duration.as_secs() / 60);
    // sleep until timer is finished
    let handle = thread::spawn(move || {
        thread::sleep(duration);
        println!("Timer finished!")
    });

    // listen for incoming messages
    thread::spawn(move || {
        let start_time = Instant::now();

        let listener = UnixListener::bind("/tmp/pomo").unwrap();

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut incoming_string = String::new();
                    stream.read_to_string(&mut incoming_string).unwrap();

                    let elapsed = start_time.elapsed();

                    let time_left = duration.saturating_sub(elapsed);

                    let response = format!("{}", time_left.as_secs());
                    stream.write_all(response.as_bytes()).unwrap();
                }

                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }
    });

    handle.join().unwrap();
}

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
fn status() {
    let mut stream = UnixStream::connect("/tmp/pomo").unwrap();

    stream.write_all(b"status").unwrap();
    stream.shutdown(Shutdown::Write).unwrap();

    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();

    println!("Time left: {}", response);
}
