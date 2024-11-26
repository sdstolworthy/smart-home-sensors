use embassy_rp::{bind_interrupts, i2c::InterruptHandler, peripherals::I2C0};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(pub struct Irqs {
    I2C0_IRQ => InterruptHandler<I2C0>;
});

/// DHT20 sensor: datasheet: https://cdn.sparkfun.com/assets/8/a/1/5/0/DHT20.pdf
pub mod dht20 {
    use defmt::debug;
    use embassy_rp::{
        i2c::{Async, I2c},
        peripherals::I2C0,
    };
    use embassy_time::Timer;
    use embedded_hal_async::i2c::I2c as I2cAsync;

    const DHT20_I2C_ADDR: u8 = 0x38;
    const DHT20_GET_STATUS: u8 = 0x71;
    const DHT20_READ_DATA: [u8; 3] = [0xAC, 0x33, 0x00];

    const DIVISOR: f32 = 2u32.pow(20) as f32;
    const TEMP_DIVISOR: f32 = DIVISOR / 200.0;

    pub async fn initialize(i2c: &mut I2c<'static, I2C0, Async>) -> bool {
        Timer::after_millis(100).await;
        let mut data = [0x0; 1];
        i2c.write_read(DHT20_I2C_ADDR, &[DHT20_GET_STATUS], &mut data)
            .await
            .expect("Can not read status");

        data[0] & 0x18 == 0x18
    }

    async fn read_data(i2c: &mut I2c<'static, I2C0, Async>) -> [u8; 6] {
        let mut data = [0x0; 6];

        for _ in 0..10 {
            i2c.write(DHT20_I2C_ADDR, &DHT20_READ_DATA)
                .await
                .expect("Can not write data");
            Timer::after_millis(80).await;

            i2c.read(DHT20_I2C_ADDR, &mut data)
                .await
                .expect("Can not read data");

            if data[0] >> 7 == 0 {
                break;
            }
        }

        data
    }

    pub async fn read_temperature_and_humidity(i2c: &mut I2c<'static, I2C0, Async>) -> (f32, f32) {
        let data = read_data(i2c).await;

        let raw_hum_data =
            ((data[1] as u32) << 12) + ((data[2] as u32) << 4) + (((data[3] & 0xf0) >> 4) as u32);
        let humidity = (raw_hum_data as f32) / DIVISOR * 100.0;

        let raw_temp_data =
            (((data[3] as u32) & 0xf) << 16) + ((data[4] as u32) << 8) + (data[5] as u32);
        let temperature = (raw_temp_data as f32) / TEMP_DIVISOR - 50.0;

        (temperature, humidity)
    }
}
