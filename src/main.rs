use argh::FromArgs;
use dirs;
use inquire::Confirm;
use pomo_cli::controller::Controller;
use pomo_cli::timer::{Timer, TimerEvent, TimerType};
use std::io::prelude::*;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::task;

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
    /// length of work period in minutes
    duration: u64,
    #[argh(option, short = 'b', default = "5")]
    /// length of break period in minutes
    break_duration: u64,
    #[argh(option, default = "4")]
    /// do a long break every nth time, set to 0 to never do a long break
    long_break_interval: u64,
    #[argh(option, default = "15")]
    /// length of long break in minutes
    long_break_duration: u64,
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
                    stream
                        .write_all(b"abort")
                        .expect("Failed to write to socket");
                    stream
                        .shutdown(std::net::Shutdown::Write)
                        .expect("Failed to shutdown socket");
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
        long_break_duration,
        long_break_interval,
    } = args;

    // multiply the durations by 60 because they are in minutes
    let duration = Duration::from_secs(duration * 60);
    let break_duration = Duration::from_secs(break_duration * 60);

    // create a new socket listener
    let listener = UnixListener::bind(SOCKET_PATH).expect("Failed to bind to socket");

    // create a new controller for running timers
    let controller_config = pomo_cli::controller::Config {
        auto,
        break_duration,
        long_break_duration: Duration::from_secs(long_break_duration),
        long_break_interval,
        work_duration: duration,
    };

    let controller = Controller::new(controller_config);

    let on_timer_finished = move |timer: &Timer| {
        run_hook("finish.sh", timer.timer_type());

        if auto {
            return;
        }

        // wait for user input
        task::spawn_blocking(move || {
            // println!("Press enter to start the next timer.");
            // let _ = std::io::stdin().read_line(&mut String::new());

            if let Ok(true) = Confirm::new("Start the next timer?")
                .with_default(true)
                .prompt()
            {
                write_socket_message("next");
            }
        });
    };

    Controller::on(&controller, TimerEvent::Start, Arc::new(on_timer_started));
    Controller::on(&controller, TimerEvent::Finish, Arc::new(on_timer_finished));
    Controller::on(&controller, TimerEvent::Pause, Arc::new(on_timer_paused));

    Controller::start(&controller);

    // listen for incoming socket messages
    let handle = tokio::task::spawn_blocking(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut incoming_string = String::new();

                    stream
                        .read_to_string(&mut incoming_string)
                        .expect("Failed to read from socket");

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
                            let timer = timer.lock().expect("Failed to lock timer");
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
                                .expect("Failed to write to socket");
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

    handle.await.expect("Failed to run socket listener");
}

fn on_timer_started(timer: &Timer) {
    run_hook("start.sh", timer.timer_type());
}

fn on_timer_paused(timer: &Timer) {
    run_hook("pause.sh", timer.timer_type());
}

fn run_hook(hook_name: &str, timer_type: TimerType) {
    let mut path = dirs::home_dir().expect("Failed to get home directory");
    path.push(HOOKS_PATH);
    path.push(Path::new(hook_name));

    // we don't care if the hook doesn't exist
    let _ = std::process::Command::new(path)
        .env("TIMER_TYPE", timer_type.to_string())
        .spawn();
}

fn cleanup() {
    // remove socket if it exists
    std::fs::remove_file(SOCKET_PATH).unwrap_or(());
}

fn write_socket_message(message: &str) -> UnixStream {
    let mut stream = match UnixStream::connect(SOCKET_PATH) {
        Ok(stream) => stream,
        Err(_) => {
            println!("Failed to connect to socket. Please start a timer using 'pomo start' first.");
            std::process::exit(exitcode::SOFTWARE);
        }
    };

    stream
        .write_all(message.as_bytes())
        .expect("Failed to write to socket");
    stream
        .shutdown(std::net::Shutdown::Write)
        .expect("Failed to shutdown socket");
    stream
}

fn pause() {
    // pause the currently running timer
    write_socket_message("pause");
}

fn resume() {
    // resume the currently paused timer
    write_socket_message("resume");
}

fn stop() {
    // stop the currently running timer
    write_socket_message("stop");
}

fn status() {
    // get the status of the currently running timer
    // write to socket and listen for response
    let mut stream = write_socket_message("status");

    let mut incoming_string = String::new();
    stream
        .read_to_string(&mut incoming_string)
        .expect("Failed to read from socket");
}

fn next() {
    // skip to the next timer
    write_socket_message("next");
}
