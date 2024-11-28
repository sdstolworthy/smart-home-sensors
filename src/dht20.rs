use embassy_rp::{bind_interrupts, i2c::InterruptHandler, peripherals::I2C0};
use {defmt_rtt as _, panic_probe as _};

/// DHT20 sensor: datasheet: https://cdn.sparkfun.com/assets/8/a/1/5/0/DHT20.pdf
use embassy_rp::i2c::{Async, I2c};
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c as I2cAsync;

pub enum ReadError {
    Unhandled,
}
const DHT20_I2C_ADDR: u8 = 0x38;
const DHT20_GET_STATUS: u8 = 0x71;
const DHT20_READ_DATA: [u8; 3] = [0xAC, 0x33, 0x00];

const DIVISOR: f32 = 2u32.pow(20) as f32;
const TEMP_DIVISOR: f32 = DIVISOR / 200.0;

pub struct Reading {
    temperature: f32,
    humidity: f32,
}

impl Reading {
    pub fn new(temperature: f32, humidity: f32) -> Self {
        Reading {
            temperature,
            humidity,
        }
    }
    pub fn celsius(&self) -> f32 {
        self.temperature
    }
    pub fn fahrenheit(&self) -> f32 {
        self.temperature * 9_f32 / 5_f32 + 32_f32
    }
    pub fn humidity(&self) -> f32 {
        self.humidity
    }
}

pub struct I2CTemperatureReader<'i2c> {
    i2c: &'i2c mut I2c<'static, I2C0, Async>,
}

impl<'i2c> I2CTemperatureReader<'i2c> {
    async fn read_data(&mut self) -> Result<[u8; 6], ReadError> {
        let mut data = [0x0; 6];

        for _ in 0..10 {
            self.i2c
                .write(DHT20_I2C_ADDR, &DHT20_READ_DATA)
                .await
                .or(Err(ReadError::Unhandled))?;
            Timer::after_millis(80).await;

            self.i2c
                .read(DHT20_I2C_ADDR, &mut data)
                .await
                .or(Err(ReadError::Unhandled))?;

            if data[0] >> 7 == 0 {
                break;
            }
        }

        Ok(data)
    }
    pub async fn initialize(i2c: &'i2c mut I2c<'static, I2C0, Async>) -> Result<Self, ()> {
        Timer::after_millis(100).await;
        let mut data = [0x0; 1];
        i2c.write_read(DHT20_I2C_ADDR, &[DHT20_GET_STATUS], &mut data)
            .await
            .expect("Can not read status");

        if data[0] & 0x18 == 0x18 {
            Ok(Self { i2c })
        } else {
            Err(())
        }
    }
    pub async fn read_temperature_and_humidity(&mut self) -> Result<Reading, ReadError> {
        let data = self.read_data().await?;

        let raw_hum_data =
            ((data[1] as u32) << 12) + ((data[2] as u32) << 4) + (((data[3] & 0xf0) >> 4) as u32);
        let humidity = (raw_hum_data as f32) / DIVISOR * 100.0;

        let raw_temp_data =
            (((data[3] as u32) & 0xf) << 16) + ((data[4] as u32) << 8) + (data[5] as u32);
        let temperature = (raw_temp_data as f32) / TEMP_DIVISOR - 50.0;

        Ok(Reading::new(temperature, humidity))
    }
}
