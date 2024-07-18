//#[macro_use]
extern crate uom;
use uom::*;

system! {
    quantities: Q {
        length: tsunit, L;
        mass: ton, M;
        time: second, T;
    }
    units: U {
        mod length::Length,
        mod mass::Mass,
        mod time::Time,
	mod velocity::Velocity,
	mod acceleration::Acceleration,
	mod gravitation::Gravitation,
	mod angularvelocity::AngularVelocity,
	mod area::Area,
	mod dimensionless::Dimensionless,
	mod specificenergy::SpecificEnergy,
    }
}

mod length {
    quantity! {
        quantity: Length; "length";
        dimension: Q<P1 /*length*/, Z0 /*mass*/, Z0 /*time*/>;
        units {
            @tsunit: 1.0e0; "tsu", "TrueSpaceunit", "TrueSpaceunits";
        }
    }
}

mod mass {
    quantity! {
        quantity: Mass; "mass";
        dimension: Q<Z0 /*length*/, P1 /*mass*/, Z0 /*time*/>;
        units {
            @ton: 1.0e0; "t", "ton", "tons";
        }
    }
}

mod time {
    quantity! {
        quantity: Time; "time";
        dimension: Q<Z0 /*length*/, Z0 /*mass*/, P1 /*time*/>;
        units {
            @second: 1.0e0; "s", "second", "seconds";
        }
    }
}

mod velocity {
    quantity! {
        quantity: Velocity; "velocity";
        dimension: Q<P1 /*length*/, Z0 /*mass*/, N1 /*time*/>;
        units {
            @tsunit_per_sec: 1.0e0; "tsu s^-1", "TrueSpaceunit per second", "TrueSpaceunits per second";
        }
    }
}

mod acceleration {
    quantity! {
        quantity: Acceleration; "acceleration";
        dimension: Q<P1 /*length*/, Z0 /*mass*/, N2 /*time*/>;
        units {
            @tsunit_per_sec_sq: 1.0e0; "tsu s^-2", "TrueSpaceunit per second squared", "TrueSpaceunits per second squared";
        }
    }
}

mod gravitation {
    quantity! {
        quantity: Gravitation; "gravitation";
        dimension: Q<P3 /*length*/, N1 /*mass*/, N2 /*time*/>;
        units {
            @tsunit_per_sec_sq_ton: 1.0e0; "tsu s^-2 t^-1", "TrueSpaceunit per second squared ton", "TrueSpaceunits per second squared ton";
            @gravitational_const: 1.1e-16; "G", "gravitational constant", "gravitational constants";
        }
    }
}

mod angularvelocity {
    quantity! {
        quantity: AngularVelocity; "angular velocity";
        dimension: Q<Z0 /*length*/, Z0 /*mass*/, N1 /*time*/>;
        units {
            @rad_per_sec: 1.0e0; "rad s^-1", "radian per second", "radians per second";
            @hertz: std::f32::consts::TAU; "Hz", "Hertz", "Hertz";
        }
    }
}

mod area {
    quantity! {
        quantity: Area; "area";
        dimension: Q<P2 /*length*/, Z0 /*mass*/, Z0 /*time*/>;
        units {
            @tsunit_sq: 1.0e0; "tsu^2", "square TrueSpaceunit", "square TrueSpaceunits";
        }
    }
}

mod dimensionless {
    quantity! {
        quantity: Dimensionless; "dimensionless";
        dimension: Q<Z0 /*length*/, Z0 /*mass*/, Z0 /*time*/>;
        units {
            @base_unit: 1.0e0; "", "", "";
        }
    }
}

mod specificenergy {
    quantity! {
        quantity: SpecificEnergy; "specific energy";
        dimension: Q<P2 /*length*/, Z0 /*mass*/, N2 /*time*/>;
        units {
            @tsu_sq_per_sec_sq: 1.0e0; "tsu^2 s^-2", "TrueSpaceunit squared per second squared", "TrueSpaceunits squared per second squared";
        }
    }
}

mod f32 {
    Q!(crate::ship::units, f32);
}

pub use self::f32::{Length, Mass, Time, Velocity, Acceleration, Gravitation, AngularVelocity, Area, SpecificEnergy};
pub use {length::tsunit, mass::ton, time::second, velocity::tsunit_per_sec, acceleration::tsunit_per_sec_sq, gravitation::tsunit_per_sec_sq_ton, gravitation::gravitational_const, angularvelocity::rad_per_sec, angularvelocity::hertz, area::tsunit_sq, specificenergy::tsu_sq_per_sec_sq};

pub fn sqrt_area(area: Area) -> Length {
    Length::new::<tsunit>(area.get::<tsunit_sq>().sqrt())
}

pub fn sqrt_spc_energy(area: SpecificEnergy) -> Velocity {
    Velocity::new::<tsunit_per_sec>(area.get::<tsu_sq_per_sec_sq>().sqrt())
}

impl From<self::f32::Dimensionless> for f32 {
    fn from(src: self::f32::Dimensionless) -> f32 {
	src.get::<dimensionless::base_unit>()
    }
}

#[macro_export]
macro_rules! make_static {
    ($unit: ident, $value: expr) => {
	$unit {dimension: std::marker::PhantomData, units: std::marker::PhantomData, value: $value}
    }
}
