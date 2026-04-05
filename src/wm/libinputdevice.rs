use crate::{river::river_libinput_device_v1::RiverLibinputDeviceV1, wm::LibinputDevice};

impl LibinputDevice {
    pub fn new(proxy: RiverLibinputDeviceV1) -> Self {
        Self {
            proxy,
            tap_support: None,
        }
    }
}
