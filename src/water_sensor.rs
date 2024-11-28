use embassy_rp::{adc::Adc, adc::Async};

pub struct WaterSensor<'adc> {
    adc: Adc<'adc, Async>,
}

impl<'adc> WaterSensor<'adc> {
    pub fn initialize(adc: Adc<'adc, Async>) -> Self {
        WaterSensor { adc }
    }
}
