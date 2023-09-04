#![no_std]

use serde_derive::{Deserialize, Serialize};

// we could use new-type pattern here but let's keep it simple
type Id = u32;
type DevId = u32;

#[derive(Debug, Serialize, Deserialize)]
#[repr(C)]
pub enum Command {
    Set(Id, Msg, DevId),
    Default,
}

#[derive(Debug, Serialize, Deserialize)]
#[repr(C)]
pub enum Msg {
    A,
    B(u32),
    C(f32), // we might consider "f16" but not sure it plays well with `ssmarshal`
}

// SET (master -> CPU_ID)
// MESSAGE1
// DEV_ID

// GET (master -> CPU_ID & CPU_ID_MASK)
// MESSAGE2
// DEV_ID
// PROTOCOL_VERSION

// SET (master -> DEV_ID)
// MESSAGE3
// PWM%
// relays[bit 0-3]

// GET (master -> DEV_ID)
// MESSAGE4
// EV_STATE[enum, not_connected/connected/err/RCD_ERROR
// pwm%
// relays[bit 0-3]
// RCD_VALUE
// Current[3] [A] float16
// Voltages[3] [V] float16
// Energy[3] [wh] float16
// BILLING_ENERGY [wh] int32
