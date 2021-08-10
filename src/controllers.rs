use anyhow::Result;
use controller_emulator::controller::ns_procon;
use controller_emulator::controller::Controller;
use controller_emulator::usb_gadget;
use std::thread::sleep;
use std::time::Duration;

pub struct NetworkControllerState([u8; 13]);

impl NetworkControllerState {
    fn get_u16(&self, offset: usize) -> u16 {
        ((self.0[offset] as u16) << 8) | (self.0[offset + 1] as u16)
    }

    pub fn player_id(&self) -> usize {
        self.0[0] as usize
    }

    pub fn _sequence_no(&self) -> u8 {
        self.0[1]
    }

    pub fn num_buttons(&self) -> usize {
        17
    }

    pub fn get_button(&self, index: usize) -> bool {
        let byte = index >> 3;
        let byte_ind = index & 7;

        ((self.0[byte] >> byte_ind) & 1) == 1
    }
    pub fn lh(&self) -> u16 {
        self.get_u16(5)
    }
    pub fn lv(&self) -> u16 {
        self.get_u16(7)
    }
    pub fn rh(&self) -> u16 {
        self.get_u16(9)
    }
    pub fn rv(&self) -> u16 {
        self.get_u16(11)
    }
}

pub trait Controllers {
    fn new(gadget_name: &str) -> Self;
    fn initialize(&mut self) -> Result<()>;
    fn set_state(&mut self, state: &NetworkControllerState);
}
pub struct NsProcons {
    gadget_name: String,
    controllers: [ns_procon::NsProcon; 4],
}

static PROCON_BUTTON_MAP: &'static [usize] = &[
    ns_procon::inputs::BUTTON_A,
    ns_procon::inputs::BUTTON_B,
    ns_procon::inputs::BUTTON_X,
    ns_procon::inputs::BUTTON_Y,
    ns_procon::inputs::BUTTON_L,
    ns_procon::inputs::BUTTON_R,
    ns_procon::inputs::BUTTON_ZL,
    ns_procon::inputs::BUTTON_ZR,
    ns_procon::inputs::BUTTON_MINUS,
    ns_procon::inputs::BUTTON_PLUS,
    ns_procon::inputs::BUTTON_L_STICK,
    ns_procon::inputs::BUTTON_R_STICK,
    ns_procon::inputs::BUTTON_UP,
    ns_procon::inputs::BUTTON_DOWN,
    ns_procon::inputs::BUTTON_LEFT,
    ns_procon::inputs::BUTTON_RIGHT,
    ns_procon::inputs::BUTTON_HOME,
];

impl Controllers for NsProcons {
    fn new(gadget_name: &str) -> Self {
        let procon_1 = ns_procon::NsProcon::create("/dev/hidg0");
        let procon_2 = ns_procon::NsProcon::create("/dev/hidg1");
        let procon_3 = ns_procon::NsProcon::create("/dev/hidg2");
        let procon_4 = ns_procon::NsProcon::create("/dev/hidg3");

        Self {
            gadget_name: gadget_name.to_string(),
            controllers: [procon_1, procon_2, procon_3, procon_4],
        }
    }

    fn initialize(&mut self) -> Result<()> {
        usb_gadget::activate(&self.gadget_name).expect("Could not activate procon gadget");

        sleep(Duration::from_secs(1));

        self.controllers[0].start_comms()?;
        self.controllers[1].start_comms()?;
        self.controllers[2].start_comms()?;
        self.controllers[3].start_comms()?;
        Ok(())
    }

    fn set_state(&mut self, state: &NetworkControllerState) {
        let controller = &mut self.controllers[state.player_id()];

        for button in 0..state.num_buttons() {
            controller.set(PROCON_BUTTON_MAP[button], state.get_button(button));
        }

        controller.set_axis(ns_procon::inputs::AXIS_LH, state.lh());
        controller.set_axis(ns_procon::inputs::AXIS_LV, state.lv());
        controller.set_axis(ns_procon::inputs::AXIS_RH, state.rh());
        controller.set_axis(ns_procon::inputs::AXIS_RV, state.rv());
    }
}
