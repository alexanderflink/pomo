use argh::FromArgs;
use dirs;
use pomo::controller::Controller;
use pomo::timer::{Timer, TimerEvent, TimerType};
use std::io::prelude::*;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

const SOCKET_PATH: &str = "/tmp/pomo.sock";
const HOOKS_PATH: &str = ".config/pomo/hooks";

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
    Resume(Resume),
    Stop(Stop),
    Status(Status),
    Next(Next),
}

#[derive(FromArgs)]
/// Start a new timer
#[argh(subcommand, name = "start")]
struct Start {
    #[argh(switch, short = 'a')]
    /// whether to automatically start the next timer when done
    auto: bool,
    #[argh(option, short = 'd', default = "25")]
    /// length of timer in minutes
    duration: u64,
    #[argh(option, short = 'b', default = "5")]
    /// length of break timer in minutes
    break_duration: u64,
}

#[derive(FromArgs)]
/// Pause a running timer
#[argh(subcommand, name = "pause")]
struct Pause {}

#[derive(FromArgs)]
/// Resume a paused timer
#[argh(subcommand, name = "resume")]
struct Resume {}

#[derive(FromArgs)]
/// Stop the currently running timer
#[argh(subcommand, name = "stop")]
struct Stop {}

#[derive(FromArgs)]
/// Get the status of the currently running timer
#[argh(subcommand, name = "status")]
struct Status {}

#[derive(FromArgs)]
/// Skip to the next timer
#[argh(subcommand, name = "next")]
struct Next {}

#[tokio::main]
async fn main() {
    let args: Args = argh::from_env();

    match args.subcommand {
        SubCommands::Start(args) => {
            start(args).await;
        }
        SubCommands::Pause(_) => pause(),
        SubCommands::Resume(_) => resume(),
        SubCommands::Stop(_) => stop(),
        SubCommands::Status(_) => status(),
        SubCommands::Next(_) => next(),
    };
}

async fn start(args: Start) {
    if std::path::Path::new(SOCKET_PATH).exists() {
        let answer = inquire::Confirm::new("Pomo is already running or was not terminated properly. Do you want to start a new pomodoro?").with_default(false).prompt();

        match answer {
            Ok(true) => {
                // if there is a running timer, send an abort message to it
                if let Ok(mut stream) = UnixStream::connect(SOCKET_PATH) {
                    stream.write_all(b"abort").unwrap();
                    stream.shutdown(std::net::Shutdown::Write).unwrap();
                }

                cleanup();
            }
            Ok(false) => {
                std::process::exit(exitcode::OK);
            }
            _ => std::process::exit(exitcode::USAGE),
        }
    }

    // handle Ctrl+C
    ctrlc::set_handler(move || {
        cleanup();
        std::process::exit(exitcode::OK);
    })
    .expect("Error setting Ctrl-C handler");

    let Start {
        auto,
        break_duration,
        duration,
    } = args;

    let duration = Duration::from_secs(duration /* * 60 */);
    let break_duration = Duration::from_secs(break_duration /* * 60 */);

    // create a new socket listener
    let listener = UnixListener::bind(SOCKET_PATH).unwrap();

    // create a new controller for running timers
    let controller = Controller::new(&duration, &break_duration, auto);

    Controller::on(&controller, TimerEvent::Start, Arc::new(on_timer_started));
    Controller::on(&controller, TimerEvent::Finish, Arc::new(on_timer_finished));

    Controller::start(&controller);

    // listen for incoming socket messages
    let handle = tokio::task::spawn_blocking(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut incoming_string = String::new();

                    stream.read_to_string(&mut incoming_string).unwrap();

                    match incoming_string.as_str() {
                        "abort" => {
                            // another instance started, abort this one
                            return;
                        }
                        "pause" => {
                            Controller::pause(&controller);
                        }
                        "resume" => {
                            Controller::start(&controller);
                        }
                        "stop" => {
                            Controller::stop(&controller);

                            cleanup();
                            // end the program when stop is called
                            return;
                        }
                        "next" => {
                            Controller::next(&controller);
                        }
                        "status" => {
                            let timer = Controller::get_current_timer(&controller);
                            let timer = timer.lock().unwrap();
                            let time_left = timer.time_left();

                            let prefix = match timer.timer_type() {
                                TimerType::Work => "W",
                                TimerType::Break => "B",
                            };

                            let minutes = time_left.as_secs() / 60;
                            let seconds = time_left.as_secs() % 60;

                            stream
                                .write_all(
                                    format!("{} {:02}:{:02}", prefix, minutes, seconds).as_bytes(),
                                )
                                .unwrap();
                        }
                        _ => {}
                    }
                }

                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }
    });

    handle.await.unwrap();
}

fn on_timer_finished(timer: &Timer) {
    let mut path = dirs::home_dir().unwrap();
    path.push(HOOKS_PATH);
    path.push(Path::new("finish.sh"));

    std::process::Command::new(path)
        .env("TIMER_TYPE", timer.timer_type().to_string())
        .spawn()
        .unwrap();
}

fn on_timer_started(timer: &Timer) {
    let mut path = dirs::home_dir().unwrap();
    path.push(HOOKS_PATH);
    path.push(Path::new("start.sh"));

    std::process::Command::new(path)
        .env("TIMER_TYPE", timer.timer_type().to_string())
        .spawn()
        .unwrap();
}

fn cleanup() {
    // remove socket if it exists
    std::fs::remove_file(SOCKET_PATH).unwrap_or(());
}

// TODO: make these functions more DRY
fn pause() {
    // pause the currently running timer
    // write to socket and listen for response
    let mut stream = UnixStream::connect(SOCKET_PATH).unwrap();
    stream.write_all(b"pause").unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();
}

fn resume() {
    // resume the currently paused timer
    // write to socket and listen for response
    let mut stream = UnixStream::connect(SOCKET_PATH).unwrap();
    stream.write_all(b"resume").unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();
}

fn stop() {
    // stop the currently running timer
    let mut stream = UnixStream::connect(SOCKET_PATH).unwrap();
    stream.write_all(b"stop").unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();
}

fn status() {
    // get the status of the currently running timer
    // write to socket and listen for response
    let mut stream = UnixStream::connect(SOCKET_PATH).unwrap();
    stream.write_all(b"status").unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();

    let mut incoming_string = String::new();
    stream.read_to_string(&mut incoming_string).unwrap();

    println!("{}", incoming_string);
}

fn next() {
    // skip to the next timer
    // write to socket and listen for response
    let mut stream = UnixStream::connect(SOCKET_PATH).unwrap();
    stream.write_all(b"next").unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();
}
