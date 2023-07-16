use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use nalgebra::Vector3;
use nix::sys::termios::BaudRate;
use planner::board::Board;
use planner::{arm::Arm, termdev::TerminalDevice};
use robby_fischer::{Command, Response};
use std::{io::Stdout, panic::AssertUnwindSafe, sync::Mutex, time::Duration};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

struct TerminalHandler {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalHandler {
    fn new() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let stdout = std::io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalHandler {
    fn drop(&mut self) {
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
        std::panic::catch_unwind(AssertUnwindSafe(|| run(&mut term_handler.terminal)))
    };

    println!();
    match res {
        Ok(ret) => {
            if let Ok(positions) = ret {
                for (i, ((a1, a2), hor)) in positions.iter().enumerate() {
                    let x_pos = Board::SQUARE_SIZE * i as f64;
                    println!("(jnp.array([{a1}, {a2}, {hor}]), jnp.array([{x_pos}, 0.0, 0.0])),");
                }
            } else {
                println!("{:?}", ret);
            }
        }
        Err(_) => {
            println!("{}", PANICINFO.lock().unwrap().as_mut().unwrap());
            std::panic::resume_unwind(Box::new(PANICINFO.lock().unwrap().take()));
        }
    }
}

fn run(_terminal: &mut Terminal<impl Backend>) -> anyhow::Result<Vec<((f64, f64), f64)>> {
    println!("starting...");
    let mut td = TerminalDevice::new("/dev/serial/by-id/usb-Raspberry_Pi_Pico_1234-if00")?;
    td.configure(BaudRate::B115200)?;
    td.set_timeout(1)?;
    let mut arm = Arm::new(td);

    println!("checking calib...");
    arm.check_calib();
    println!("calib check done!");
    arm.bottom_angle_offset = 0.0;
    arm.top_angle_offset = 0.0;
    arm.translation_offset = Vector3::new(0.0, 0.0, 0.0);

    let mut current_square = 0;
    let mut positions = Vec::new();
    let mut theta1 = 50.0;
    let mut theta2 = 50.0;
    arm.claw_pos.y = 0.01;

    println!("Getting currenst position...");
    arm.send_command(Command::Position).unwrap();
    let response = arm.get_response().unwrap();
    if let Response::Position(old_hor, old_theta1, old_theta2) = response {
        theta1 = old_theta1 as f64;
        theta2 = old_theta2 as f64;
        arm.claw_pos.y = old_hor as f64;
    } else {
        println!("expected position");
    }
    println!("Got current position.");

    let mut changed = true;
    println!("Move to row {current_square}");
    loop {
        if let Ok(true) = event::poll(Duration::from_millis(1)) {
            let event = event::read()?;
            let step_size = 0.8;
            if let Event::Key(key) = event {
                match key.code {
                    KeyCode::Char('p') => {
                        println!("{:?}", Arm::arm_2d_angles(arm.claw_pos));
                    }
                    KeyCode::Enter => {
                        positions.push((Arm::arm_2d_angles(arm.claw_pos), arm.claw_pos.y));
                        current_square += 1;
                        if current_square == 8 {
                            break;
                        }
                        println!("Move to row {current_square}");
                    }
                    KeyCode::Esc => {
                        break;
                    }
                    KeyCode::Char('a') => {
                        arm.claw_pos.y -= 0.002;
                        changed = true;
                    }
                    KeyCode::Char('t') => {
                        arm.claw_pos.y += 0.002;
                        changed = true;
                    }
                    KeyCode::Char('u') => {
                        positions.pop();
                        current_square = 0.max(current_square - 1);
                    }
                    KeyCode::Left => {
                        theta1 -= step_size;
                        changed = true;
                    }
                    KeyCode::Right => {
                        theta1 += step_size;
                        changed = true;
                    }
                    KeyCode::Up => {
                        theta2 -= step_size;
                        changed = true;
                    }
                    KeyCode::Down => {
                        theta2 += step_size;
                        changed = true;
                    }
                    _ => {}
                }
            }
            if changed {
                let new_claw_pos2d = Arm::position_from_angles(theta1, theta2);
                let new_claw_pos =
                    Vector3::new(new_claw_pos2d[0], arm.claw_pos.y, new_claw_pos2d[1]);
                arm.move_claw_to(new_claw_pos);
                changed = false;
            }
        }
    }
    Ok(positions)
}
