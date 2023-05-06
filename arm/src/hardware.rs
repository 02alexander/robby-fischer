use core::fmt::Write;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use alloc::string::String;
use alloc::vec::Vec;
use cortex_m::delay::Delay;
use embedded_alloc::Heap;
use rp_pico::hal::clocks::init_clocks_and_plls;
use rp_pico::hal::clocks::UsbClock;
use rp_pico::hal::pwm::Slices;
use rp_pico::hal::usb::UsbBus;
use rp_pico::hal::{Clock, Sio, Timer, Watchdog};
use rp_pico::pac::{interrupt, Interrupt, NVIC, RESETS, USBCTRL_DPRAM, USBCTRL_REGS};
use rp_pico::pac::{CorePeripherals, Peripherals};
use rp_pico::Pins;
use usb_device::bus::UsbBusAllocator;
use usb_device::device::{UsbDevice, UsbDeviceBuilder, UsbVidPid};
use usbd_serial::SerialPort;

// USB device
static mut USB_DEVICE: Option<UsbDevice<UsbBus>> = None;
static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;
static mut USB_SERIAL: Option<SerialPort<UsbBus>> = None;

// Read buffer
static mut READ_BUFFER: [u8; 4096] = [0; 4096];
static READ_AVAILABLE: AtomicUsize = AtomicUsize::new(0);

// Set to `true` when writing should retry.
static WRITE_AVAILABLE: AtomicBool = AtomicBool::new(true);

// The heap allocator.
#[global_allocator]
static HEAP: Heap = Heap::empty();

// The heap data, 128KiB.
// To do: Figure out how much is actually available.
const HEAP_SIZE: usize = 0x20000;
static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];

/// The entry point. Sets up the hardware.
#[cortex_m_rt::entry]
fn entry() -> ! {
    // Initialize the heap
    unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_MEM.len()) }

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
        start_serial(
            pac.USBCTRL_REGS,
            pac.USBCTRL_DPRAM,
            clocks.usb_clock,
            &mut pac.RESETS,
        );
    }

    let delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);

    let sio = Sio::new(pac.SIO);

    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let slices = Slices::new(pac.PWM, &mut pac.RESETS);

    super::start(delay, timer, pins, slices);
}

/// Starts the serial communication.
///
/// # Safety
///
/// May only be called once, and must be called before any of the other funcitons in this module.
pub unsafe fn start_serial(
    regs: USBCTRL_REGS,
    dpram: USBCTRL_DPRAM,
    clock: UsbClock,
    resets: &mut RESETS,
) {
    // Set up the USB driver.
    let bus = USB_BUS.insert(UsbBusAllocator::new(UsbBus::new(
        regs, dpram, clock, true, resets,
    )));

    // Set up the serial port.
    USB_SERIAL = Some(SerialPort::new(bus));

    // Create a USB device (with a fake ID and info)
    USB_DEVICE = Some(
        UsbDeviceBuilder::new(bus, UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("Raspberry Pi")
            .product("Pico")
            .serial_number("1234")
            .device_class(2)
            .build(),
    );

    // Enable the interrupt.
    NVIC::unmask(Interrupt::USBCTRL_IRQ);
}

/// USB interrupt.
#[interrupt]
#[allow(non_snake_case)]
unsafe fn USBCTRL_IRQ() {
    let usb_dev = USB_DEVICE.as_mut().unwrap();
    let serial = USB_SERIAL.as_mut().unwrap();

    // Poll the device, and return if nothing more needs to be done.
    if !usb_dev.poll(&mut [serial]) {
        return;
    }

    // Tell the writer that it may write again, in case it failed.
    WRITE_AVAILABLE.store(true, Ordering::Relaxed);

    // Discard reads if the buffer is full.
    let index = READ_AVAILABLE.load(Ordering::Acquire);
    if index >= READ_BUFFER.len() {
        _ = serial.read(&mut [0; 64]);
    }

    let buffer = &mut READ_BUFFER[index..];

    match serial.read(buffer) {
        Ok(0) | Err(_) => {}
        Ok(count) => {
            let index = index + count;
            READ_AVAILABLE.store(index, Ordering::Release);
        }
    }
}

/// Writes the data to the USB serial.
pub fn serial_write(mut data: &[u8]) {
    while !data.is_empty() {
        // Wait until writing is available.
        while !WRITE_AVAILABLE.load(Ordering::Relaxed) {
            core::hint::spin_loop();
        }

        // Write as much as possible to the device.
        let count = cortex_m::interrupt::free(|_| {
            let serial = unsafe { USB_SERIAL.as_mut().unwrap() };
            match serial.write(data) {
                Ok(0) | Err(_) => {
                    WRITE_AVAILABLE.store(false, Ordering::Relaxed);
                    0
                }
                Ok(len) => len,
            }
        });

        data = &data[count..];
    }
}

/// Waits until any data is available and then reads from the serial device. The
/// closure must return the amount of consumed bytes.
pub fn serial_read(handler: impl FnOnce(&[u8]) -> usize) {
    // Ensure it's not called recursively.
    static CALLED: AtomicBool = AtomicBool::new(false);
    assert!(!CALLED.load(Ordering::Relaxed));
    CALLED.store(true, Ordering::Relaxed);

    // Wait until data is available.
    while READ_AVAILABLE.load(Ordering::Relaxed) == 0 {
        core::hint::spin_loop();
    }

    cortex_m::interrupt::free(|_| {
        let index = READ_AVAILABLE.load(Ordering::Acquire);
        let data = unsafe { &mut READ_BUFFER[..index] };

        let consumed = handler(data).min(index);

        unsafe { READ_BUFFER.copy_within(consumed..index, 0) };
        READ_AVAILABLE.store(index - consumed, Ordering::Release);
    });

    CALLED.store(false, Ordering::Relaxed);
}

#[cfg(not(test))]
#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    println!("{info}");
    loop {
        core::hint::spin_loop();
    }
}

pub struct SerialPrinter;
impl Write for SerialPrinter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        serial_write(s.as_bytes());
        Ok(())
    }
}

#[allow(unused)]
macro_rules! print {
    ($($args:tt)+) => {{
        use $crate::hardware::SerialPrinter;
        use ::core::fmt::Write;
        write!(SerialPrinter, $($args)+).unwrap();
    }};
}
#[allow(unused)]
macro_rules! println {
    () => {{
        $crate::hardware::serial_write(b"\r\n");
    }};
    ($($args:tt)+) => {{
        $crate::hardware::print!($($args)+);
        $crate::hardware::serial_write(b"\r\n");
    }};
}

#[allow(unused)]
pub(super) use {print, println};

/// Tests if any data is available on serial.
pub fn serial_available() -> bool {
    READ_AVAILABLE.load(Ordering::Relaxed) > 0
}

/// Reads until the specified byte is found. Returns the results as a `String`.
#[allow(unused)]
pub fn read_until(byte: u8) -> String {
    let mut buf = Vec::new();
    let mut terminated = false;

    while !terminated {
        serial_read(|data| {
            if let Some(i) = data.iter().position(|&b| b == byte) {
                terminated = true;
                buf.extend_from_slice(&data[..=i]);
                i + 1
            } else {
                buf.extend_from_slice(data);
                data.len()
            }
        })
    }

    String::from_utf8(buf)
        .unwrap_or_else(|err| String::from_utf8_lossy(err.as_bytes()).into_owned())
}

/// Reads a single byte.
#[allow(unused)]
pub fn read_byte() -> u8 {
    let mut byte = None;

    loop {
        if let Some(byte) = byte {
            return byte;
        }

        serial_read(|data| {
            byte = data.first().copied();
            data.len().min(1)
        });
    }
}
