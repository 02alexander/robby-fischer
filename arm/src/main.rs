#![no_std]
#![no_main]

extern crate alloc;

mod hardware;
mod stepper;

use core::{str::FromStr, f32};

use robby_fischer::{Command, Response};
use alloc::{collections::VecDeque, string::{String, ToString}};
use cortex_m::delay::Delay;
use hardware::read_byte;
use rp_pico::hal::Timer;
use rp_pico::hal::{
    gpio::DynPin,
    pwm::{Channel, ChannelId, SliceId, SliceMode, Slices},
};
use rp_pico::Pins;
use stepper::{Direction, StepSize, Stepper};

use crate::hardware::{println, serial_available};

const TOP_RATIO: f32 =  66.0/ 21.0; // Stepper angle / arm angle
const BOT_RATIO: f32 = (34.0/8.0)*(54.0/10.0); // Stepper angle / arm angle
const SIDEWAYS_DEGREE_PER_M: f32 = 360.0 / (18.0 * 0.002); 

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
        println!("waiting for sideways");
        self.sideways_stepper.calibrate(&mut self.sideways_button, 20.0, 500., delay);

        println!("waiting for bottom arm");
        self.bottom_arm_stepper.calibrate(&mut self.bottom_arm_button, 20.0, 1000., delay);
        self.bottom_arm_stepper.goto_angle(200.);

        println!("waiting for top arm");
        self.top_arm_stepper.calibrate(&mut self.top_arm_button, 20.0, 200., delay);
    }

    pub fn is_calibrated(&self) -> bool {
        self.is_calibrated
    }

    pub fn parse_command(&mut self, delay: &mut Delay, line: &str) {
        if let Ok(command) = Command::from_str(line) {
            println!("{:?}", command);
            match  command {
                Command::Calibrate => {
                    self.calibrate(delay);
                },
                Command::MoveSideways(angle) => {
                    self.sideways_stepper.goto_angle(angle * SIDEWAYS_DEGREE_PER_M);
                },
                Command::MoveTopArm(angle) => {
                    self.top_arm_stepper.goto_angle(angle * TOP_RATIO);
                },
                Command::MoveBottomArm(angle) => {
                    self.bottom_arm_stepper.goto_angle(angle * BOT_RATIO);                    
                },
                Command::Queue(a1, a2, a3) => {
                    self.movement_buffer.push_back((a1, a2, a3));
                },
                Command::QueueSize => {
                    println!("{}",Response::QueueSizeResponse(self.movement_buffer.len() as u32, 12).to_string());
                },
                Command::Position => {
                    println!("{} {} {} {} {} {}", 
                        self.sideways_stepper.cur_pos, self.sideways_stepper.target_pos, 
                        self.bottom_arm_stepper.cur_pos, self.bottom_arm_stepper.target_pos, 
                        self.top_arm_stepper.cur_pos, self.top_arm_stepper.target_pos);
                }
            }
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

    let top_arm_stepper = Stepper::new(
        DynPin::from(pins.gpio12.into_push_pull_output()),
        DynPin::from(pins.gpio11.into_push_pull_output()),
        StepSize::DIV16,
        Direction::Clockwise,
        Some((
            DynPin::from(pins.gpio15.into_push_pull_output()),
            DynPin::from(pins.gpio14.into_push_pull_output()),
            DynPin::from(pins.gpio13.into_push_pull_output()),
        )),
    );
    
    let bottom_arm_stepper = Stepper::new(
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
    let mut line_buffer = String::with_capacity(4096);

    let mut arm = Arm {
        top_arm_stepper,
        sideways_stepper,
        bottom_arm_stepper,

        bottom_arm_button: DynPin::from(pins.gpio17.into_pull_up_input()),
        sideways_button: DynPin::from(pins.gpio16.into_pull_up_input()),
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
