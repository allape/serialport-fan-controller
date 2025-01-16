use std::{ffi::CStr, thread, time::Duration};

use anyhow::{anyhow, Result};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        self,
        gpio::OutputPin,
        ledc::{
            config::TimerConfig, LedcChannel, LedcDriver, LedcTimer, LedcTimerDriver, Resolution,
        },
        peripheral::Peripheral,
        prelude::*,
    },
    nvs::EspDefaultNvsPartition,
    sys::{esp_err_t, esp_err_to_name, ESP_ERR_TIMEOUT},
};
use log::{error, info, warn};

pub fn esp_err_to_str(err: esp_err_t) -> &'static str {
    cstr_to_str(unsafe { esp_err_to_name(err) }).unwrap_or("Unknown ESP Error")
}

pub fn cstr_to_str(cstr: *const i8) -> Result<&'static str> {
    if cstr.is_null() {
        return Err(anyhow!("null"));
    }
    unsafe { Ok(CStr::from_ptr(cstr).to_str()?) }
}

pub fn new_pwm<'a, Timer, Channel>(
    timer: impl Peripheral<P = Timer> + 'a,
    channel: impl Peripheral<P = Channel> + 'a,
    pin: impl Peripheral<P = impl OutputPin> + 'a,
    duty: Option<u32>,
    frequency: Option<Hertz>,
    resolution: Option<Resolution>,
) -> Result<LedcDriver<'a>>
where
    Timer: LedcTimer + 'a,
    Channel: LedcChannel<SpeedMode = Timer::SpeedMode>,
{
    let mut config = TimerConfig::default();
    config.frequency = frequency.unwrap_or(Hertz(10_000));
    config.resolution = resolution.unwrap_or(Resolution::Bits8);

    let timer_driver = LedcTimerDriver::new(timer, &config)?;
    let mut ledc_driver = LedcDriver::new(channel, &timer_driver, pin)?;

    ledc_driver.set_duty(duty.unwrap_or(0))?;

    Ok(ledc_driver)
}

struct PwmDriver<'a> {
    led: LedcDriver<'a>,
    pwm: LedcDriver<'a>,
}

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let _sysloop = EspSystemEventLoop::take()?;
    let _nvs = EspDefaultNvsPartition::take()?;

    #[cfg(feature = "esp-c3-32s")]
    let mut output = PwmDriver {
        led: new_pwm(
            peripherals.ledc.timer0,
            peripherals.ledc.channel0,
            peripherals.pins.gpio5, // blue led
            None,
            None,
            None,
        )?,
        pwm: new_pwm(
            peripherals.ledc.timer1,
            peripherals.ledc.channel1,
            peripherals.pins.gpio3, // red led
            Some(256),
            None,
            None,
        )?,
    };

    #[cfg(feature = "esp32-c3-supermini")]
    let mut output = PwmDriver {
        led: new_pwm(
            peripherals.ledc.timer0,
            peripherals.ledc.channel0,
            peripherals.pins.gpio8, // built-in led
            Some(256),
            None,
            None,
        )?,
        pwm: new_pwm(
            peripherals.ledc.timer1,
            peripherals.ledc.channel1,
            peripherals.pins.gpio3,
            None,
            None,
            None,
        )?,
    };

    info!("Startting serial loop...");

    let max_duty = output.pwm.get_max_duty();

    // AsyncUartDriver not working properly
    #[cfg(feature = "esp-c3-32s")]
    let serial = hal::uart::UartDriver::new(
        peripherals.uart0,
        peripherals.pins.gpio21,
        peripherals.pins.gpio20,
        Option::<hal::gpio::AnyIOPin>::None,
        Option::<hal::gpio::AnyIOPin>::None,
        &Default::default(),
    )?;

    #[cfg(feature = "esp32-c3-supermini")]
    let mut serial = hal::usb_serial::UsbSerialDriver::new(
        peripherals.usb_serial,
        peripherals.pins.gpio18,
        peripherals.pins.gpio19,
        &Default::default(),
    )?;

    let mut string_buf = String::new();
    let mut read_buf = [0u8; 100];

    info!("Serial Fan Controller is up and running!");

    loop {
        #[cfg(feature = "esp32-c3-supermini")]
        if !serial.is_connected() {
            info!("USB Serial not connected, waiting 3 seconds...");
            thread::sleep(Duration::from_secs(3));
            continue;
        }

        let n = match serial.read(&mut read_buf, 20 / 1000) {
            Ok(n) => n,
            Err(e) => {
                if e.code() == ESP_ERR_TIMEOUT {
                    info!("Timeout reading from serial, waiting 3 seconds...");
                    thread::sleep(Duration::from_secs(3));
                } else {
                    error!("Error reading from serial: {:?}", e);
                }
                0
            }
        };

        if n == 0 {
            #[cfg(feature = "esp32-c3-supermini")]
            {
                info!("Timeout reading from serial, waiting 3 seconds...");
                thread::sleep(Duration::from_secs(3));
            }
            continue;
        }

        let input = std::str::from_utf8(&read_buf[..n]).unwrap();

        info!("Received {} bytes: {:?}", n, &input);

        if input.ends_with("\n") || input.ends_with("\r") {
            string_buf.push_str(input);
        } else {
            string_buf.push_str(input);
            continue;
        }

        let input = string_buf.clone();
        let input = input.trim();

        let speed = if input.contains("\r") {
            input.split("\r")
        } else {
            input.split("\n")
        };
        let speed = speed.last().unwrap_or("").trim();

        string_buf.clear();

        if speed.is_empty() {
            warn!("Empty speed, skipping...");
            continue;
        }

        let duty = speed.parse::<u32>().unwrap_or(0);

        #[cfg(feature = "esp32-c3-supermini")]
        {
            output.led.set_duty(max_duty - duty).unwrap();
            output.pwm.set_duty(duty).unwrap();
        }
        #[cfg(feature = "esp-c3-32s")]
        {
            output.led.set_duty(duty).unwrap();
            output.pwm.set_duty(max_duty - duty).unwrap();
        }

        info!("Set duty to {}", duty);
    }

    // info!("Serial Fan Controller is up and running!");
    // loop {
    //     thread::sleep(time::Duration::from_secs(1));
    // }

    // Ok(())
}
