use embedded_hal::digital::v2::OutputPin;
use rp_pico::hal::Timer;

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

#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Clockwise,
    CounterClockwise,
}

#[derive(Debug)]
pub struct Stepper<SP, DP, TMS1, TMS2, TMS3> {
    step_size: StepSize,
    step_pin: SP,
    step_is_high: bool,
    dir_pin: DP,

    cur_pos: i32, // In number of sixtenth steps
    pub target_pos: i32,

    time_us_last_step: u32,
    step_time_us: u32,
    positive_direction: Direction,
    cur_direction: Direction,
    mode_pins: Option<(TMS1, TMS2, TMS3)>,
}

impl<SP: OutputPin, DP: OutputPin, TMS1: OutputPin, TMS2: OutputPin, TMS3: OutputPin>
    Stepper<SP, DP, TMS1, TMS2, TMS3>
{
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
        step_pin: SP,
        dir_pin: DP,
        step_size: StepSize,
        positive_direction: Direction,
        mode_pins: Option<(TMS1, TMS2, TMS3)>,
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
            step_time_us: 400,
            cur_direction: positive_direction,
            positive_direction,
        };
        stepper.set_step_size(step_size);
        stepper
    }

    pub fn goto_angle(&mut self, angle: f32) {
        self.goto_position(
            (self.step_size as u8 as f32 * (angle / STEPS_PER_REVOLUTION as f32)) as i32,
        );
    }

    pub fn set_direction(&mut self, direction: Direction) {
        match direction {
            Direction::Clockwise => {
                let _ = self.dir_pin.set_high();
            }
            Direction::CounterClockwise => {
                let _ = self.dir_pin.set_low();
            }
        }
    }

    pub fn goto_position(&mut self, position: i32) {
        self.target_pos = position;
        if self.target_pos < self.cur_pos {
            self.set_direction(!self.positive_direction);
            self.cur_direction = !self.positive_direction;
        }
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

    pub fn get_step_time(&self) -> u32 {
        self.step_time_us
    }

    pub fn step(&mut self) {
        if self.step_is_high {
            let _ = self.step_pin.set_low();
            self.step_is_high = false;
            self.cur_pos += 1;
        } else {
            let _ = self.step_pin.set_high();
            self.step_is_high = true;
        }
    }

    pub fn run(&mut self, timer: &Timer) {
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
