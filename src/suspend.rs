use std::io;
use std::io::stdout;
use nix::sys::signal::Signal;
use nix::unistd::Pid;

/// suspend the process
pub fn suspend() -> io::Result<()> {
    crossterm::execute!(stdout(), crossterm::terminal::Clear(crossterm::terminal::ClearType::All))?;
    crate::tui::restore()?;

    // send SIGSTOP to ourselves
    nix::sys::signal::kill(Pid::from_raw(0), Signal::SIGSTOP)?;

    crate::tui::setup()?;
    crate::TERMINAL.get().unwrap().lock().unwrap().clear()?;
    Ok(())
}
