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
use debugless_unwrap::{DebuglessUnwrap};
use embedded_hal::{blocking::delay::DelayMs, blocking::i2c, digital::v2::InputPin, PwmPin};
use fugit::RateExtU32;
use hardware::read_byte;
use mlx90393::{DigitalFilter, I2CInterface, Magnetometer, OverSamplingRatio};
use robby_fischer::{Command, Response};
use rp_pico::hal::{pwm, Timer};
use rp_pico::hal::{Clock, Sio, I2C};
use rp_pico::Pins;
use rp_pico::{
    hal::{
        clocks::init_clocks_and_plls,
        gpio::DynPin,
        pwm::{Channel, ChannelId, SliceId, SliceMode, Slices},
        rom_data::reset_to_usb_boot,
        Watchdog,
    },
    pac::CorePeripherals,
    pac::Peripherals,
};
use stepper::{Direction, StepSize, Stepper};

use crate::hardware::{println, serial_available};

// const TOP_RATIO: f32 = 66.0 / 21.0; // Stepper angle / arm angle
const TOP_RATIO: f32 = 66.0 / 20.0; // Why does this work better?
const BOT_RATIO: f32 = (34.0 / 8.0) * (54.0 / 10.0); // Stepper angle / arm angle
const SIDEWAYS_DEGREE_PER_M: f32 = 360.0 / (18.0 * 0.002);

const BOT_ARM_MAX_SPEED: f32 = 1200.0;
const TOP_ARM_MAX_SPEED: f32 = 120.0;
const SIDEWAYS_MAX_SPEED: f32 = 1600.0;

struct AngleSensor {
    pub mlx: Magnetometer,
    address: u8,
}

impl AngleSensor {
    pub fn new<WR, E>(i2c: &mut WR, address: u8) -> Result<Self, mlx90393::Error<E>>
    where
        WR: i2c::WriteRead<Error = E>,
    {
        let mut protocol = I2CInterface { i2c, address };
        let mut mlx = Magnetometer::default_settings(&mut protocol)?;
        mlx.set_filter(&mut protocol, DigitalFilter::DF4)?;
        mlx.set_oversampling_ratio(&mut protocol, OverSamplingRatio::OSR4)?;

        // mlx.
        Ok(AngleSensor { mlx, address })
    }

    pub fn get_angle<WR, E>(
        &mut self,
        i2c: &mut WR,
        delay: &mut impl DelayMs<u32>,
    ) -> Result<f32, mlx90393::Error<E>>
    where
        WR: i2c::WriteRead<Error = E>,
    {
        let mut protocol = I2CInterface {
            i2c,
            address: self.address,
        };
        let (_t, x, y, _z) = self.mlx.do_measurement(&mut protocol, delay)?;
        // println!("{} {}", x, y);
        let angle = libm::atan2f(y as f32, x as f32);
        Ok(angle * 180. / core::f32::consts::PI)
    }
}

struct Arm<S: SliceId, M: SliceMode, C: ChannelId, I> {
    is_sideways_calibrated: bool,

    i2c: I,
    bottom_angle_sensor: AngleSensor,
    top_angle_sensor: AngleSensor,

    bottom_arm_stepper: Stepper,

    top_arm_stepper: Stepper,

    sideways_stepper: Stepper,
    sideways_button: DynPin,
    chess_button: DynPin,
    chess_button_last_state: bool,
    chess_button_been_pressed: bool,

    servo_channel: Channel<S, M, C>,

    movement_buffer: VecDeque<(f32, f32, f32, f32)>,
}

impl<S: SliceId, M: SliceMode, I> Arm<S, M, pwm::B, I>
where
    I: i2c::WriteRead,
{
    pub fn calibrate_sideways(&mut self, delay: &mut Delay) {
        self.sideways_stepper
            .calibrate(&mut self.sideways_button, 20.0, 500., delay);

        self.is_sideways_calibrated = true;
    }

    pub fn calibrate_arm(&mut self, delay: &mut Delay) -> Option<()> {
        // Constant error on bottom sensor, might be because the sensor is
        // not aligned perfectly with the magnet.

        let mut a1 = self
            .bottom_angle_sensor
            .get_angle(&mut self.i2c, delay)
            .ok()?;
        a1 += 90.0;
        if a1 < 0.0 {
            a1 += 360.0
        };
        let mut a2 = self.top_angle_sensor.get_angle(&mut self.i2c, delay).ok()?;
        a2 += 2.0;
        a2 = -a2;
        a2 -= 90.0;
        if a2 < 0.0 {
            a2 += 360.0
        };

        // println!("{a1} {a2}");
        self.bottom_arm_stepper.calib_real_angle(a1 * BOT_RATIO);
        self.top_arm_stepper
            .calib_real_angle((a2 + a1 / TOP_RATIO) * TOP_RATIO);
        Some(())
    }

    pub fn parse_command(&mut self, delay: &mut Delay, line: &str) {
        if let Ok(command) = Command::from_str(line) {
            match command {
                Command::Magnets => {
                    let mut a1 = self
                        .bottom_angle_sensor
                        .get_angle(&mut self.i2c, delay)
                        .debugless_unwrap();
                    a1 += 90.0;
                    if a1 < 0.0 {
                        a1 += 360.0
                    };
                    let mut a2 = self
                        .top_angle_sensor
                        .get_angle(&mut self.i2c, delay)
                        .debugless_unwrap();
                    a2 += 2.0;
                    a2 = -a2;
                    a2 -= 90.0;
                    if a2 < 0.0 {
                        a2 += 360.0
                    };
                    
                    println!(
                        "{}",
                        Response::Magnets(a1, a2)
                    );
                }
                Command::CalibrateArm => {
                    self.calibrate_arm(delay);
                }
                Command::CalibrateSideways => {
                    self.calibrate_sideways(delay);
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
                Command::Queue(a1, a2, sd, speed_scale_factor) => {
                    self.movement_buffer.push_back((
                        a1 * BOT_RATIO,
                        (a2 + a1 / TOP_RATIO) * TOP_RATIO,
                        sd * SIDEWAYS_DEGREE_PER_M,
                        speed_scale_factor,
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
                            self.top_arm_stepper.get_angle() / TOP_RATIO
                                - (self.bottom_arm_stepper.get_angle() / BOT_RATIO) / TOP_RATIO,
                            self.sideways_stepper.get_angle() / SIDEWAYS_DEGREE_PER_M,
                        )
                        .to_string()
                    );
                }
                Command::IsCalibrated => {
                    println!(
                        "{}",
                        Response::IsCalibrated(self.is_sideways_calibrated).to_string()
                    );
                }
                Command::Grip => {
                    self.servo_channel.set_duty(1000 + 370);
                }
                Command::Release => {
                    self.servo_channel.set_duty(1000);
                }
                Command::RestartToBoot => {
                    reset_to_usb_boot(0, 0);
                }
                Command::ChessButton => {
                    println!(
                        "{}",
                        Response::ChessButtonStatus(self.chess_button_been_pressed)
                    );
                    self.chess_button_been_pressed = false;
                }
            }
        }
    }

    fn is_in_position_margin(&mut self, margin: i64) -> bool {
        self.top_arm_stepper.is_at_target_margin(margin)
            && self.bottom_arm_stepper.is_at_target_margin(margin)
            && self.sideways_stepper.is_at_target_margin(margin)
    }

    fn check_queue(&mut self) {
        if !self.movement_buffer.is_empty() && self.is_in_position_margin(3){
            let (a1, a2, sd, speed_scale_factor) = self.movement_buffer.pop_front().unwrap();
            let speed_scale_factor = (1.0_f32).min(speed_scale_factor);
            let max_time = ((libm::fabsf(self.bottom_arm_stepper.get_angle() - a1)
                / BOT_ARM_MAX_SPEED)
                .max(libm::fabsf(self.top_arm_stepper.get_angle() - a2) / TOP_ARM_MAX_SPEED)
                .max(libm::fabsf(self.sideways_stepper.get_angle() - sd) / SIDEWAYS_MAX_SPEED)
                + 0.0001)
                / speed_scale_factor;

            // let norma1 = libm::fabsf(self.bottom_arm_stepper.get_angle() - a1)/max_time;
            // let norma2 = libm::fabsf(self.top_arm_stepper.get_angle() - a2)/max_time;
            // let normsd = libm::fabsf(self.sideways_stepper.get_angle() - sd)/max_time;

            self.bottom_arm_stepper
                .set_velocity((self.bottom_arm_stepper.get_angle() - a1) / max_time);
            self.top_arm_stepper
                .set_velocity((self.top_arm_stepper.get_angle() - a2) / max_time);
            self.sideways_stepper
                .set_velocity((self.sideways_stepper.get_angle() - sd) / max_time);

            self.bottom_arm_stepper.goto_angle(a1);
            self.top_arm_stepper.goto_angle(a2);
            self.sideways_stepper.goto_angle(sd);
        }
    }

    pub fn run(&mut self, timer: &Timer) {
        let pressed = self.chess_button.is_low().unwrap();
        if !self.chess_button_last_state && pressed {
            self.chess_button_been_pressed = true;
        }
        self.chess_button_last_state = pressed;

        self.check_queue();
        self.sideways_stepper.run(timer);
        self.bottom_arm_stepper.run(timer);
        self.top_arm_stepper.run(timer);
    }
}

fn start() -> ! {
    // Hardware setup.
    let mut pac = Peripherals::take().unwrap();
    let core = CorePeripherals::take().unwrap();

    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    let clocks = init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    unsafe {
        hardware::start_serial(
            pac.USBCTRL_REGS,
            pac.USBCTRL_DPRAM,
            clocks.usb_clock,
            &mut pac.RESETS,
        );
    }

    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);

    let sio = Sio::new(pac.SIO);

    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let slices = Slices::new(pac.PWM, &mut pac.RESETS);

    let mut i2c = I2C::i2c1(
        pac.I2C1,
        pins.gpio26.into_mode(),
        pins.gpio27.into_mode(),
        400.kHz(),
        &mut pac.RESETS,
        125_000_000.Hz(),
    );

    let mut pwm = slices.pwm1;
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

    let bottom_angle_sensor = AngleSensor::new(&mut i2c, 0x18).debugless_unwrap();
    // bottom_angle_sensor.mlx.set_gain(&mut I2CInterface {i2c: &mut i2c, address: 0x18}, Gain::X1).debugless_unwrap();
    let top_angle_sensor = AngleSensor::new(&mut i2c, 0x19).debugless_unwrap();

    let mut arm = Arm {
        i2c,
        top_arm_stepper,
        sideways_stepper,
        bottom_arm_stepper,

        bottom_angle_sensor,
        top_angle_sensor,

        chess_button: DynPin::from(pins.gpio22.into_pull_up_input()),
        chess_button_been_pressed: false,
        chess_button_last_state: false,
        sideways_button: DynPin::from(pins.gpio16.into_pull_up_input()),

        is_sideways_calibrated: false,
        servo_channel: channel,
        movement_buffer: VecDeque::new(),
    };

    println!("{:+?}", arm.calibrate_arm(&mut delay));

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
