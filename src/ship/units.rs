
mod internal {
    make_units! {
	UQM;
	ONE: Unitless;

	base {
            TSU: TrueSpaceUnit, "tsu", Length;
            S: Second, "s", Time;
            T: Ton, "t", Mass;
	}

	derived {
	    TSU2: TrueSpaceUnit2 = (TrueSpaceUnit * TrueSpaceUnit), Area;
            TSUpS: TrueSpaceUnitPerSecond = (TrueSpaceUnit / Second), Velocity;
            TSUpS2: TrueSpaceUnitPerSecond2 = (TrueSpaceUnit / Second / Second), Acceleration;
            GRAVUNIT: GravitationUnit = (TrueSpaceUnit * TrueSpaceUnit * TrueSpaceUnit / Ton / Second / Second);
            RADpS: RadianPerSecond = (Unitless / Second);
            RADpS2: RadianPerSecond2 = (Unitless / Second / Second);
	    TTSU2: TonTrueSpaceUnit2 = (TrueSpaceUnit2 * Ton);// Moment of Inertia
	}

	constants {
            G: GravitationUnit = 1.1e-16;
            TAU: Unitless = consts::TAU;
	    HZ: RadianPerSecond = consts::TAU;
	}

	fmt = true;
    }
}

pub use internal::f32consts::*;
pub use internal::{Unitless, TrueSpaceUnit, Second, Ton, TrueSpaceUnitPerSecond, TrueSpaceUnitPerSecond2, RadianPerSecond, RadianPerSecond2, TonTrueSpaceUnit2};
