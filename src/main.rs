#![no_std]
#![no_main]

extern crate alloc;

mod hardware;
mod stepper;

use alloc::{collections::VecDeque, string::String};
use cortex_m::delay::Delay;
use embedded_hal::{
    digital::v2::{InputPin, OutputPin},
    PwmPin,
};
use hardware::read_byte;
use rp_pico::hal::Timer;
use rp_pico::hal::{
    gpio::DynPin,
    pwm::{Channel, ChannelId, SliceId, SliceMode, Slices},
};
use rp_pico::Pins;
use stepper::{Direction, StepSize, Stepper};

use crate::hardware::{println, read_until, serial_available};

struct Arm<S: SliceId, M: SliceMode, C: ChannelId> {
    is_calibrated: bool,
    
    bottom_arm_stepper: Stepper,
    bottom_arm_button: DynPin,

    top_arm_stepper: Stepper,
    top_arm_button: DynPin,

    sideways_stepper: Stepper,
    sideways_button: DynPin,

    servo_channel: Channel<S, M, C>,

    movement_buffer: VecDeque<(f32, f32, f32)>,
}

impl<S: SliceId, M: SliceMode, C: ChannelId> Arm<S, M, C> {

    pub fn calibrate(&mut self, delay: &mut Delay) {
        self.sideways_stepper.calibrate(&mut self.sideways_button, 20.0, 1000., delay);

        self.bottom_arm_stepper.calibrate(&mut self.bottom_arm_button, 20.0, 500., delay);
        self.bottom_arm_stepper.goto_angle(200.);

        self.top_arm_stepper.calibrate(&mut self.top_arm_button, 20.0, 500., delay);
    }

    pub fn is_calibrated(&self) -> bool {
        self.is_calibrated
    }

    pub fn parse_command(&mut self, delay: &mut Delay, line: &str) {
        if let Ok(vel) = line.trim().parse::<f32>() {
            println!("set velocity to {}", vel);
            self.bottom_arm_stepper.set_velocity(vel);
            println!("{}", self.bottom_arm_stepper.get_step_time());
        } else if line.starts_with("a") {
            if let Ok(angle) = line.trim().parse::<f32>() {
                self.bottom_arm_stepper.goto_angle(angle);
            }
        } else if line.starts_with("c") {
            println!("calibrating...");
            self.calibrate(delay);
            println!("done calibrating");
        } else {
            println!("got {}", line.trim());
        }
    
    }
    
    pub fn run(&mut self, timer: &Timer) {
        self.sideways_stepper.run(&timer);
        self.bottom_arm_stepper.run(&timer);
        self.top_arm_stepper.run(&timer);

        
    }

}

fn start(mut delay: Delay, timer: Timer, pins: Pins, pwm_slices: Slices) -> ! {
    let mut pwm = pwm_slices.pwm1;
    pwm.set_ph_correct();
    pwm.set_div_int(20);
    pwm.enable();    // let channel = &mut pwm.channel_a;
    // channel.output_to(pins.pins.gpio12.into_push_pull_output()gpio2);

    let mut channel = pwm.channel_b;
    channel.output_to(pins.gpio19.into_push_pull_output());

    let bottom_arm_stepper = Stepper::new(
        DynPin::from(pins.gpio12.into_push_pull_output()),
        DynPin::from(pins.gpio11.into_push_pull_output()),
        StepSize::DIV16,
        Direction::CounterClockwise,
        Some((
            DynPin::from(pins.gpio15.into_push_pull_output()),
            DynPin::from(pins.gpio14.into_push_pull_output()),
            DynPin::from(pins.gpio13.into_push_pull_output()),
        )),
    );
    
    let top_arm_stepper = Stepper::new(
        DynPin::from(pins.gpio7.into_push_pull_output()),
        DynPin::from(pins.gpio6.into_push_pull_output()),
        StepSize::DIV16,
        Direction::Clockwise,
        Some((
            DynPin::from(pins.gpio10.into_push_pull_output()),
            DynPin::from(pins.gpio9.into_push_pull_output()),
            DynPin::from(pins.gpio8.into_push_pull_output()),
        )),
    );

    let sideways_stepper = Stepper::new(
        DynPin::from(pins.gpio2.into_push_pull_output()),
        DynPin::from(pins.gpio1.into_push_pull_output()),
        StepSize::DIV16,
        Direction::CounterClockwise,
        Some((
            DynPin::from(pins.gpio5.into_push_pull_output()),
            DynPin::from(pins.gpio4.into_push_pull_output()),
            DynPin::from(pins.gpio3.into_push_pull_output()),
        )),
    );



    // let button_pin = pins.gpio16.into_pull_up_input();

    let mut line_buffer = String::with_capacity(4096);



    let mut arm = Arm {
        top_arm_stepper,
        sideways_stepper,
        bottom_arm_stepper,

        bottom_arm_button: DynPin::from(pins.gpio16.into_pull_up_input()),
        sideways_button: DynPin::from(pins.gpio17.into_pull_up_input()),
        top_arm_button: DynPin::from(pins.gpio18.into_pull_up_input()),

        is_calibrated: false,
        servo_channel:  channel,
        movement_buffer: VecDeque::new(),  
    };

    loop {
        arm.run(&timer);


        while serial_available() {
            let ch = char::from(read_byte());
            line_buffer.push(ch);
            if line_buffer.len() >= 4096 {
                println!("ERR line_buffer too large");
                line_buffer.clear();
            }
            if ch == '\n' {
                arm.parse_command(&mut delay, &line_buffer);
                line_buffer.clear();
            }
            // Read and write serial.
        }
    }
}
