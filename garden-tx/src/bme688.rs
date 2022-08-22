use core::sync::atomic::AtomicBool;

use atsamd_hal::clock::Tc4Tc5Clock;
use atsamd_hal::pac::{PM, TC4};
use atsamd_hal::sleeping_delay::SleepingDelay;
use atsamd_hal::timer::{TimerCounter, TimerCounter4};
use drogue_bme680::{
    Bme680Controller, Bme680Sensor, Configuration, DelayMsWrapper, StaticProvider,
};
use feather_m0::I2c;
use garden_shared::BME688SensorReport;
use uom::si::electrical_resistance::ohm;
use uom::si::f32::ElectricalResistance;
use uom::si::f32::{Pressure, Ratio, ThermodynamicTemperature};
use uom::si::pressure::pascal;
use uom::si::ratio::percent;
use uom::si::thermodynamic_temperature::degree_celsius;

pub struct Bme688 {
    bme: Bme680Controller<I2c, DelayMsWrapper<SleepingDelay<TimerCounter4>>, StaticProvider>,
    last: Option<BME688SensorReport>,
}

pub static TC4_FIRED: AtomicBool = AtomicBool::new(false);

impl Bme688 {
    pub fn new(i2c: I2c, tc4: TC4, tc45: &Tc4Tc5Clock, pm: &mut PM) -> Self {
        let timer = TimerCounter::tc4_(tc45, tc4, pm);
        let delay = DelayMsWrapper::new(SleepingDelay::new(timer, &TC4_FIRED));

        let bme = Bme680Sensor::from(i2c, drogue_bme680::Address::Primary).unwrap();

        let mut config = Configuration::standard();
        config.run_gas = false;

        let controller = Bme680Controller::new(bme, delay, config, StaticProvider(14)).unwrap();

        Self {
            bme: controller,
            last: None,
        }
    }

    pub fn read(&mut self) -> Option<BME688SensorReport> {
        let result = self.bme.measure_default().ok()??;

        let report = BME688SensorReport {
            temp: ThermodynamicTemperature::new::<degree_celsius>(result.temperature - 5.0),
            pressure: Pressure::new::<pascal>(result.pressure.unwrap_or(0.0)),
            humidity: Ratio::new::<percent>(result.humidity),
            gas_resistance: ElectricalResistance::new::<ohm>(result.gas_resistance),
        };

        let report = report.sanity_check(self.last.as_ref())?;
        self.last = Some(report.clone());
        Some(report)
    }
}
