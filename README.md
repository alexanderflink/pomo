# Pomo
A small command line utility for running Pomodoro timers. Uses sockets instead of polling a file each time you check the timer.

## Installation
### Using cargo
`cargo install pomo`

## Commands
`pomo start`
Start a new timer

Options:

  -a, --auto            whether to automatically start the next timer when done

  -d, --duration        length of work period in minutes

  -b, --break-duration  length of break period in minutes

  --long-break-interval do a long break every nth time, set to 0 to never do a long break

  --long-break-duration length of long break in minutes

  --help                display usage information

`pomo pause`
Pause a running timer

`pomo resume`
Resume a paused timer

`pomo stop`
Stop the currently running timer

`pomo status`
Get the status of the currently running timer. Prints the timer time as W for Work timer and B for Break timer, along with the minutes and seconds left.

`pomo next`
Skip to the next timer without finishing the current one.

## Todo
- [ ] Write tests
