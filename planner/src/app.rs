use std::{time::Duration, fs::File};
use anyhow::anyhow;
use crossterm::event::{self, Event, KeyCode};
use nalgebra::Vector3;
use nix::sys::termios::BaudRate;
use robby_fischer::{Command, Response};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};

use crate::{arm::Arm, termdev::TerminalDevice};

fn find_possible_tty_dev() -> Option<String> {
    for dir_entry in std::fs::read_dir("/dev/").ok()? {
        let dir_entry = dir_entry.ok()?;
        let os_file_name = dir_entry.file_name();
        let file_name = os_file_name.to_string_lossy();
        if file_name.starts_with("tty")
            && file_name.len() >= 6
            && (&file_name[3..6] == "USB" || &file_name[3..6] == "ACM")
        {
            return Some("/dev/".to_string() + &file_name);
        }
    }
    None
}


pub fn run(_terminal: &mut Terminal<impl Backend>) -> anyhow::Result<()> {
    let ttyfile = find_possible_tty_dev().ok_or(anyhow!("Found no terminal device"))?;
    let mut td = TerminalDevice::new(ttyfile)?;
    td.configure(BaudRate::B115200)?;
    let mut arm = Arm::new(td);    
    
    arm.send_command(Command::IsCalibrated).unwrap();
    let response = arm.get_response().unwrap();
    if response != Response::IsCalibrated(true) {
        arm.send_command(Command::Calibrate).unwrap();
    }

    arm.move_claw_to(Vector3::new(0.35, 0.5, 0.01));
    loop {
        // terminal.draw(|b| {
        //     let chunks = Layout::default()
        //         .direction(Direction::Vertical)
        //         .constraints(vec![Constraint::Length(3), Constraint::Min(10)])
        //         .split(b.size());
        // })?;
        

        if let Ok(true) = event::poll(Duration::from_millis(1)) {
            let event = event::read()?;
            let step_size = 0.002;
            match event {
                Event::Key(key) => match key.code {
                    KeyCode::Char('a') => {
                        let mut state = arm.state;
                        state.file = (state.file+1).min(15);
                        arm.raw_move(state)?;
                    },
                    KeyCode::Char('t') => {
                        let mut state = arm.state;
                        state.file = (state.file as i32-1).max(0) as u8;
                        arm.raw_move(state)?;
                    },
                    KeyCode::Char('p') => {
                        println!("{:?}", Arm::angles(arm.claw_pos));
                    },
                    
                    KeyCode::Esc => {
                        break;
                    }
                    KeyCode::Left => {
                        arm.move_claw(Vector3::new(-step_size, 0.0, 0.0));
                    }
                    KeyCode::Right => {
                        arm.move_claw(Vector3::new(step_size, 0.0, 0.0));
                    }
                    KeyCode::Up => {
                        arm.move_claw(Vector3::new(0.0, 0.0, step_size));
                    }
                    KeyCode::Down => {
                        arm.move_claw(Vector3::new(0.0, 0.0, -step_size));
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
    Ok(())
}
