#![no_std]
#![no_main]

extern crate alloc;

mod hardware;
mod stepper;

use cortex_m::delay::Delay;
use embedded_hal::{
    digital::v2::{InputPin, OutputPin},
    PwmPin,
};
use rp_pico::hal::pwm::Slices;
use rp_pico::hal::Timer;
use rp_pico::Pins;
use stepper::{Direction, StepSize, Stepper};

use crate::hardware::{println, read_until, serial_available};

fn start(mut delay: Delay, timer: Timer, pins: Pins, pwm_slices: Slices) -> ! {
    // let mut pwm = pwm_slices.pwm1;
    // pwm.set_ph_correct();
    // pwm.set_div_int(20);
    // pwm.enable();

    // let channel = &mut pwm.channel_a;
    // channel.output_to(pins.pins.gpio12.into_push_pull_output()gpio2);

    let mut stepper = Stepper::new(
        pins.gpio12.into_push_pull_output(),
        pins.gpio11.into_push_pull_output(),
        StepSize::DIV16,
        Direction::Clockwise,
        Some((
            pins.gpio15.into_push_pull_output(),
            pins.gpio14.into_push_pull_output(),
            pins.gpio13.into_push_pull_output(),
        )),
    );

    let button_pin = pins.gpio16.into_pull_up_input();

    loop {
        stepper.run(&timer);

        if serial_available() {
            // Read and write serial.
            let line = read_until(b'\n');
            if let Ok(vel) = line.trim().parse::<f32>() {
                println!("set velocity to {}", vel);
                stepper.set_velocity(vel);
                println!("{}", stepper.get_step_time());
            } else if line.starts_with("a") {
                if let Ok(angle) = line.trim().parse::<f32>() {
                    stepper.goto_angle(angle);
                }
            } else {
                println!("got {}", line.trim());
            }
        }
    }
}
