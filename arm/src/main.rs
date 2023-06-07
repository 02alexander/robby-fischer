#![no_std]
#![no_main]

extern crate alloc;

mod hardware;
mod stepper;

use core::{f32, str::FromStr};

use alloc::{
    collections::VecDeque,
    string::{String, ToString},
};
use cortex_m::delay::Delay;
use embedded_hal::PwmPin;
use hardware::read_byte;
use robby_fischer::{Command, Response};
use rp_pico::hal::{
    gpio::DynPin,
    pwm::{Channel, ChannelId, SliceId, SliceMode, Slices},
};
use rp_pico::hal::{pwm, Timer};
use rp_pico::Pins;
use stepper::{Direction, StepSize, Stepper};

use crate::hardware::{println, serial_available};

const TOP_RATIO: f32 = 66.0 / 21.0; // Stepper angle / arm angle
const BOT_RATIO: f32 = (34.0 / 8.0) * (54.0 / 10.0); // Stepper angle / arm angle
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

    movement_buffer: VecDeque<(f32, f32, f32, f32)>,
}

impl<S: SliceId, M: SliceMode> Arm<S, M, pwm::B> {
    pub fn calibrate(&mut self, delay: &mut Delay) {
        self.sideways_stepper
            .calibrate(&mut self.sideways_button, 20.0, 500., delay);

        self.top_arm_stepper
            .calibrate(&mut self.top_arm_button, 20.0, 200., delay);

        self.bottom_arm_stepper
            .calibrate(&mut self.bottom_arm_button, 20.0, 1000., delay);
        self.bottom_arm_stepper.goto_angle(200.);

        self.top_arm_stepper
            .calibrate(&mut self.top_arm_button, 20.0, 200., delay);
        self.is_calibrated = true;
    }

    pub fn parse_command(&mut self, delay: &mut Delay, line: &str) {
        if let Ok(command) = Command::from_str(line) {
            match command {
                Command::Calibrate => {
                    self.calibrate(delay);
                }
                Command::MoveSideways(angle) => {
                    self.sideways_stepper.set_velocity(800.0);
                    self.sideways_stepper
                        .goto_angle(angle * SIDEWAYS_DEGREE_PER_M);
                }
                Command::MoveTopArm(angle) => {
                    self.top_arm_stepper.set_velocity(150.0);
                    self.top_arm_stepper.goto_angle(angle * TOP_RATIO);
                }
                Command::MoveBottomArm(angle) => {
                    self.bottom_arm_stepper.set_velocity(600.0);
                    self.bottom_arm_stepper.goto_angle(angle * BOT_RATIO);
                }
                Command::Queue(a1, a2, sd ,speed_scale_factor) => {
                    self.movement_buffer.push_back((
                        a1 * BOT_RATIO,
                        a2 * TOP_RATIO,
                        sd * SIDEWAYS_DEGREE_PER_M,
                        speed_scale_factor
                    ));
                }
                Command::QueueSize => {
                    println!(
                        "{}",
                        Response::QueueSize(self.movement_buffer.len() as u32, 300).to_string()
                    );
                }
                Command::Position => {
                    println!(
                        "{}",
                        Response::Position(
                            self.bottom_arm_stepper.get_angle() / BOT_RATIO,
                            self.top_arm_stepper.get_angle() / TOP_RATIO,
                            self.sideways_stepper.get_angle() / SIDEWAYS_DEGREE_PER_M,
                        )
                        .to_string()
                    );
                }
                Command::IsCalibrated => {
                    println!("{}", Response::IsCalibrated(self.is_calibrated).to_string());
                }
                Command::Grip => {
                    self.servo_channel.set_duty(1000 + 300);
                }
                Command::Release => {
                    self.servo_channel.set_duty(1000);
                }
            }
        }
    }

    fn is_in_position_margin(&mut self, margin: i64) -> bool {
        return self.top_arm_stepper.is_at_target_margin(margin)
            && self.bottom_arm_stepper.is_at_target_margin(margin)
            && self.sideways_stepper.is_at_target_margin(margin);
    }

    fn check_queue(&mut self) {
        if self.movement_buffer.len() > 0 {
            if self.is_in_position_margin(3) {
                const BOT_ARM_MAX_SPEED: f32 = 1600.0;
                const TOP_ARM_MAX_SPEED: f32 = 200.0;
                const SIDEWAYS_MAX_SPEED: f32 = 1600.0;

                let (a1, a2, sd, speed_scale_factor) = self.movement_buffer.pop_front().unwrap();
                let speed_scale_factor = (1.0_f32).min(speed_scale_factor);
                let max_time = ((libm::fabsf(self.bottom_arm_stepper.get_angle() - a1) / BOT_ARM_MAX_SPEED)
                    .max(libm::fabsf(self.top_arm_stepper.get_angle() - a2) / TOP_ARM_MAX_SPEED)
                    .max(libm::fabsf(self.sideways_stepper.get_angle() - sd) / SIDEWAYS_MAX_SPEED )+0.0001)/speed_scale_factor;
                
                // let norma1 = libm::fabsf(self.bottom_arm_stepper.get_angle() - a1)/max_time;
                // let norma2 = libm::fabsf(self.top_arm_stepper.get_angle() - a2)/max_time;
                // let normsd = libm::fabsf(self.sideways_stepper.get_angle() - sd)/max_time;

                self.bottom_arm_stepper.set_velocity((self.bottom_arm_stepper.get_angle() - a1) / max_time);
                self.top_arm_stepper.set_velocity((self.top_arm_stepper.get_angle() - a2) / max_time);
                self.sideways_stepper.set_velocity((self.sideways_stepper.get_angle() - sd) / max_time);

                self.bottom_arm_stepper.goto_angle(a1);
                self.top_arm_stepper.goto_angle(a2);
                self.sideways_stepper.goto_angle(sd);

            }
        }
    }

    pub fn run(&mut self, timer: &Timer) {
        self.check_queue();
        self.sideways_stepper.run(&timer);
        self.bottom_arm_stepper.run(&timer);
        self.top_arm_stepper.run(&timer);
    }
}

fn start(mut delay: Delay, timer: Timer, pins: Pins, pwm_slices: Slices) -> ! {
    let mut pwm = pwm_slices.pwm1;
    pwm.set_ph_correct();
    pwm.set_div_int(120); // 120 MHz / 120 = 1000 kHz
    pwm.set_top(20000); // 1000 kHz / 20000 = 50 Hz
    pwm.enable(); // let channel = &mut pwm.channel_a;

    let mut channel = pwm.channel_b;
    channel.output_to(pins.gpio19.into_push_pull_output());

    // loop {
    //     channel.set_duty(1000);
    //     delay.delay_ms(2000);
    //     channel.set_duty(2000);
    //     delay.delay_ms(2000);
    // }

    let mut top_arm_stepper = Stepper::new(
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

    let mut bottom_arm_stepper = Stepper::new(
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

    let mut sideways_stepper = Stepper::new(
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

    bottom_arm_stepper.set_velocity(360.0);
    top_arm_stepper.set_velocity(50.0);
    sideways_stepper.set_velocity(180.0);
    let mut line_buffer = String::with_capacity(4096);

    let mut arm = Arm {
        top_arm_stepper,
        sideways_stepper,
        bottom_arm_stepper,

        bottom_arm_button: DynPin::from(pins.gpio17.into_pull_up_input()),
        sideways_button: DynPin::from(pins.gpio16.into_pull_up_input()),
        top_arm_button: DynPin::from(pins.gpio18.into_pull_up_input()),

        is_calibrated: false,
        servo_channel: channel,
        movement_buffer: VecDeque::new(),
    };

    loop {
        arm.run(&timer);

        while serial_available() {

            // So it dosn't wait too long between runs.
            arm.run(&timer);

            let ch = char::from(read_byte());
            line_buffer.push(ch);
            if line_buffer.len() >= 4096 {
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
