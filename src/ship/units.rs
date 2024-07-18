//pub use self::f32::{Length, Mass, Time, Velocity, Acceleration, Gravitation, AngularVelocity, Area, SpecificEnergy};

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
	    ERGpT: ErgPerTon = (TrueSpaceUnitPerSecond * TrueSpaceUnitPerSecond);
            TSUpS2: TrueSpaceUnitPerSecond2 = (TrueSpaceUnit / Second / Second), Acceleration;
            GRAVUNIT: GravitationUnit = (TrueSpaceUnit * TrueSpaceUnit * TrueSpaceUnit / Ton / Second / Second);
            RADpS: RadianPerSecond = (Unitless / Second), Frequency;// not really frequency
	}

	constants {
            G: GravitationUnit = 1.1e-16;
            TAU: Unitless = consts::TAU;
	    RpS: RadianPerSecond = consts::TAU;
	}

	fmt = true;
    }
}

pub use internal::f32consts::*;
pub use internal::{TrueSpaceUnit, Second, Ton, TrueSpaceUnit2, TrueSpaceUnitPerSecond, ErgPerTon, TrueSpaceUnitPerSecond2, RadianPerSecond, Unitless};
