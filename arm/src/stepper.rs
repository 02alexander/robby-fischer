use cortex_m::delay::Delay;
use debugless_unwrap::DebuglessUnwrap;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use rp_pico::hal::{gpio::DynPin, Timer};

const STEPS_PER_REVOLUTION: u32 = 200;
const MAX_REVOLUTIONS_PER_SECOND: f32 = 6.25;
const MAX_VELOCITY: f32 = MAX_REVOLUTIONS_PER_SECOND * STEPS_PER_REVOLUTION as f32;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum StepSize {
    DIV1 = 1,
    DIV2 = 2,
    DIV4 = 4,
    DIV8 = 8,
    DIV16 = 16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Clockwise,
    CounterClockwise,
}

pub struct Stepper {
    step_size: StepSize,
    step_pin: DynPin,
    step_is_high: bool,
    dir_pin: DynPin,

    pub cur_pos: i64, // In number of sixtenth steps
    pub target_pos: i64,

    time_us_last_step: u32,
    step_time_us: u32,
    pub positive_direction: Direction,
    pub cur_direction: Direction,
    mode_pins: Option<(DynPin, DynPin, DynPin)>,
}

impl Stepper {
    pub fn set_step_size(&mut self, step_size: StepSize) {
        self.step_size = step_size;
        if let Some((ms1, ms2, ms3)) = &mut self.mode_pins {
            match self.step_size {
                StepSize::DIV1 => {
                    let _ = ms1.set_low();
                    let _ = ms2.set_low();
                    let _ = ms3.set_low();
                }
                StepSize::DIV2 => {
                    let _ = ms1.set_high();
                    let _ = ms2.set_low();
                    let _ = ms3.set_low();
                }
                StepSize::DIV4 => {
                    let _ = ms1.set_low();
                    let _ = ms2.set_high();
                    let _ = ms3.set_low();
                }
                StepSize::DIV8 => {
                    let _ = ms1.set_high();
                    let _ = ms2.set_high();
                    let _ = ms3.set_low();
                }
                StepSize::DIV16 => {
                    let _ = ms1.set_high();
                    let _ = ms2.set_high();
                    let _ = ms3.set_high();
                }
            }
        }
    }

    pub fn new(
        step_pin: DynPin,
        dir_pin: DynPin,
        step_size: StepSize,
        positive_direction: Direction,
        mode_pins: Option<(DynPin, DynPin, DynPin)>,
    ) -> Self {
        let mut stepper = Stepper {
            step_size,
            step_pin,
            dir_pin,
            mode_pins,
            cur_pos: 0,
            target_pos: 0,
            step_is_high: false,
            time_us_last_step: 0,
            step_time_us: 4000,
            cur_direction: positive_direction,
            positive_direction,
        };

        // Write to the directions pin.
        stepper.set_direction(stepper.cur_direction);

        stepper.set_step_size(step_size);
        stepper
    }

    pub fn calibrate<P: InputPin>(
        &mut self,
        button_pin: &mut P,
        slow_velocity: f32,
        fast_velocity: f32,
        delay: &mut Delay,
    ) {
        let old_step_time = self.step_time_us;
        self.set_velocity(fast_velocity);
        self.set_direction(!self.positive_direction);
        while button_pin.is_high().debugless_unwrap() {
            self.step();
            delay.delay_us(self.step_time_us);
        }
        self.set_velocity(slow_velocity);
        self.set_direction(self.positive_direction);
        while button_pin.is_low().debugless_unwrap() {
            self.step();
            delay.delay_us(self.step_time_us);
        }
        self.step_time_us = old_step_time;
        self.cur_pos = 0;
    }

    /// Gets angle in degrees from start.
    pub fn get_angle(&self) -> f32 {
        self.cur_pos as f32 / self.step_size as u8 as f32 / STEPS_PER_REVOLUTION as f32 * 360.
    }

    /// Recalibrate the stepper by giving it it's current angle in degrees.
    pub fn calib_real_angle(&mut self, angle: f32) {
        let real_pos = (self.step_size as u8 as f32 * (angle / 360. * STEPS_PER_REVOLUTION as f32)) as i64;
        self.cur_pos = real_pos;
        self.target_pos = real_pos;

    }

    /// Goto angles in degrees.
    pub fn goto_angle(&mut self, angle: f32) {
        self.goto_position(
            (self.step_size as u8 as f32 * (angle / 360. * STEPS_PER_REVOLUTION as f32)) as i64,
        );
    }

    pub fn set_direction(&mut self, direction: Direction) {
        self.cur_direction = direction;
        match direction {
            Direction::Clockwise => {
                let _ = self.dir_pin.set_high();
            }
            Direction::CounterClockwise => {
                let _ = self.dir_pin.set_low();
            }
        }
    }

    pub fn goto_position(&mut self, position: i64) {
        self.target_pos = position;
    }

    pub fn is_at_target_margin(&self, margin: i64) -> bool {
        (self.cur_pos - self.target_pos).abs() <= margin
    }

    /// Velocity in degrees per second, always positive.
    pub fn set_velocity(&mut self, velocity: f32) {
        let mut velocity = libm::fabsf(velocity);
        if velocity > MAX_VELOCITY {
            velocity = MAX_VELOCITY;
        }
        let full_steps_per_second = velocity * (STEPS_PER_REVOLUTION as f32 / 360.0);
        let steps_per_second = (self.step_size as u8 as f32) * full_steps_per_second;
        self.step_time_us = (1e6 / steps_per_second) as u32 / 2; // /2 because it waits for switch high and for switch low,
    }

    pub fn step(&mut self) {
        if self.step_is_high {
            let _ = self.step_pin.set_low();
            self.step_is_high = false;
            self.cur_pos += if self.cur_direction == self.positive_direction {
                1
            } else {
                -1
            };
        } else {
            let _ = self.step_pin.set_high();
            self.step_is_high = true;
        }
    }

    pub fn run(&mut self, timer: &Timer) {
        if self.target_pos < self.cur_pos {
            self.set_direction(!self.positive_direction);
        } else {
            self.set_direction(self.positive_direction);
        }
        if self.target_pos != self.cur_pos {
            let cur_time = timer.get_counter().ticks() as u32;
            if cur_time - self.time_us_last_step > self.step_time_us {
                self.step();
                self.time_us_last_step = cur_time;
            }
        }
    }
}

impl core::ops::Not for Direction {
    type Output = Direction;

    fn not(self) -> Self::Output {
        match self {
            Direction::Clockwise => Direction::CounterClockwise,
            Direction::CounterClockwise => Direction::Clockwise,
        }
    }
}
