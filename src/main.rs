#![no_std]
#![no_main]

use core::cell::RefCell;

use defmt::*;
use display_interface_spi::SPIInterface;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_rp::adc::{Adc, Config as AdcConfig};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::i2c::{self, Config as I2cConfig};
use embassy_rp::peripherals::I2C0;
use embassy_rp::spi::Spi;
use embassy_rp::{bind_interrupts, spi};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_time::Delay;
use embassy_time::Timer;
use embedded_graphics::image::{Image, ImageRawLE};
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use mipidsi::models::ST7789;
use mipidsi::options::{Orientation, Rotation};
use mipidsi::Builder;
use water_sensor::WaterSensor;
use {defmt_rtt as _, panic_probe as _};

mod dht20;
mod water_sensor;

bind_interrupts!(pub struct AdcIrqs {
    ADC_IRQ_FIFO => embassy_rp::adc::InterruptHandler;
});
bind_interrupts!(pub struct I2cIrqs {
    I2C0_IRQ => embassy_rp::i2c::InterruptHandler<I2C0>;
});
const DISPLAY_FREQ: u32 = 64_000_000;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    info!("Hello World!");

    let bl = p.PIN_25;
    let rst = p.PIN_12;
    let display_cs = p.PIN_9;
    let dcx = p.PIN_8;
    let mosi = p.PIN_11;
    let clk = p.PIN_10;

    // create SPI
    let mut display_config = spi::Config::default();
    display_config.frequency = DISPLAY_FREQ;
    display_config.phase = spi::Phase::CaptureOnSecondTransition;
    display_config.polarity = spi::Polarity::IdleHigh;

    let spi = Spi::new_blocking_txonly(p.SPI1, clk, mosi, display_config.clone());
    let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi));

    let display_spi = SpiDeviceWithConfig::new(
        &spi_bus,
        Output::new(display_cs, Level::High),
        display_config,
    );

    let dcx = Output::new(dcx, Level::Low);
    let rst = Output::new(rst, Level::Low);
    // dcx: 0 = command, 1 = data

    // Enable LCD backlight
    let _bl = Output::new(bl, Level::High);

    // display interface abstraction from SPI and DC
    let di = SPIInterface::new(display_spi, dcx);

    // Define the display from the display interface and initialize it
    let mut display = Builder::new(ST7789, di)
        .display_size(135, 240)
        .orientation(Orientation::new().rotate(Rotation::Deg270))
        .invert_colors(mipidsi::options::ColorInversion::Inverted)
        .reset_pin(rst)
        .display_offset(50, 40)
        //        .orientation(Orientation::new().rotate(Rotation::Deg270))
        .init(&mut Delay)
        .unwrap();
    display.clear(Rgb565::RED).unwrap();

    let raw_image_data = ImageRawLE::new(include_bytes!("../assets/ferris.raw"), 86);
    let ferris = Image::new(&raw_image_data, Point::new(34, 68));

    // Display the image
    ferris.draw(&mut display).unwrap();

    let style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
    let sda = p.PIN_28;
    let scl = p.PIN_29;

    let mut i2c = i2c::I2c::new_async(p.I2C0, scl, sda, I2cIrqs, I2cConfig::default());
    let mut reader = dht20::I2CTemperatureReader::initialize(&mut i2c)
        .await
        .unwrap();

    let mut adc = Adc::new(p.ADC, AdcIrqs, AdcConfig::default());
    let mut water_sensor = WaterSensor::initialize(adc);

    loop {
        let _ = display.clear(Rgb565::BLACK);
        let _ = Text::new("Temperature:", Point::new(0, 20), style).draw(&mut display);
        let _ = Text::new("Humidity:", Point::new(0, 80), style).draw(&mut display);
        if let Ok(reading) = reader.read_temperature_and_humidity().await {
            let mut celsius_buffer = itoa::Buffer::new();
            let celsius = celsius_buffer.format(reading.celsius() as i64);
            let mut fahrenheit_buffer = itoa::Buffer::new();
            let fahrenheit = fahrenheit_buffer.format(reading.fahrenheit() as i64);
            let mut humidity_buffer = itoa::Buffer::new();
            let humidity = humidity_buffer.format(reading.humidity() as i64);
            let _ = Text::new(celsius, Point::new(0, 40), style).draw(&mut display);
            let _ = Text::new(fahrenheit, Point::new(0, 60), style).draw(&mut display);
            let _ = Text::new(humidity, Point::new(0, 100), style).draw(&mut display);
        }
        Timer::after_millis(1000).await;
    }
}
