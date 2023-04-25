/**
* `main` parses the command line arguments (start, pause, stop, status) and runs other functions
* accordingly
*/
fn main() {}

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
