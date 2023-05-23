mod app;
mod arm;
mod termdev;

use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::{io::Stdout, panic::AssertUnwindSafe, sync::Mutex};
use tui::{backend::CrosstermBackend, Terminal};

struct TerminalHandler {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalHandler {
    fn new() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let stdout = std::io::stdout();
        // execute!(
        //     stdout, 
        //     // EnterAlternateScreen, 
        // )?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalHandler {
    fn drop(&mut self) {
        // Cleanup.
        // let _ = execute!(
        //     self.terminal.backend_mut(),
        //     // LeaveAlternateScreen,
        // );
        let _ = disable_raw_mode();
        let _ = self.terminal.show_cursor();
    }
}

static PANICINFO: Mutex<Option<String>> = Mutex::new(None);

fn main() {
    std::panic::set_hook(Box::new(|e| {
        let mut info = PANICINFO.lock().unwrap();
        *info = Some(format!("{:?}", e));
    }));

    let res = {
        let mut term_handler = TerminalHandler::new().unwrap();
        std::panic::catch_unwind(AssertUnwindSafe(|| {
            app::run(&mut term_handler.terminal)
        }))
    };

    match res {
        Ok(e) => {
            println!("{:?}", e);
        }
        Err(_) => {
            println!("{}", PANICINFO.lock().unwrap().as_mut().unwrap());
            std::panic::resume_unwind(Box::new(PANICINFO.lock().unwrap().take()));
        }
    }
}
