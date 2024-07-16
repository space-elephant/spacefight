use std::ops::{Add, Mul, AddAssign, MulAssign, Neg, Sub, SubAssign, Div, DivAssign};

macro_rules! mkunit {
    ($unit: ty) => {
	impl From<f32> for $unit {
	    fn from(src: f32) -> Self {
		Self(src)
	    }
	}

	impl From<$unit> for f32 {
	    fn from(src: $unit) -> f32 {
		src.0
	    }
	}
	
	impl Neg for $unit {
	    type Output = Self;

	    fn neg(self) -> Self {
		Self(-self.0)
	    }
	}
	
	impl Add for $unit {
	    type Output = Self;

	    fn add(self, other: Self) -> Self {
		Self(self.0 + other.0)
	    }
	}

	impl AddAssign for $unit {
	    fn add_assign(&mut self, other: Self) {
		*self = *self + other;
	    }
	}
	
	impl Sub for $unit {
	    type Output = Self;

	    fn sub(self, other: Self) -> Self {
		self + (-other)
	    }
	}

	impl SubAssign for $unit {
	    fn sub_assign(&mut self, other: Self) {
		*self = *self - other;
	    }
	}

	multiply!($unit, f32, $unit);

	impl MulAssign<f32> for $unit {
	    fn mul_assign(&mut self, other: f32) {
		*self = *self * other;
	    }
	}

	impl DivAssign<f32> for $unit {
	    fn div_assign(&mut self, other: f32) {
		*self = *self / other;
	    }
	}
    };
}

macro_rules! base_mul {
    ($base: ty, $other: ty, $result: ty) => {
	impl Mul<$other> for $base {
	    type Output = $result;

	    fn mul(self, other: $other) -> $result {
		(f32::from(self) * f32::from(other)).into()
	    }
	}

	impl Div<$other> for $result {
	    type Output = $base;

	    fn div(self, other: $other) -> $base {
		(f32::from(self) / f32::from(other)).into()
	    }
	}
    };
}

macro_rules! multiply {
    ($base: ty, $other: ty, $result: ty) => {
	base_mul!($base, $other, $result);
	base_mul!($other, $base, $result);
    };
    ($base: ty, $square: ty) => {
	base_mul!($base, $base, $square);
	
	impl $base {
	    pub fn sqr(self) -> $square {
		self * self
	    }
	}

	impl $square {
	    pub fn sqrt(self) -> $base {
		f32::from(self).sqrt().into()
	    }
	}
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Time(pub f32);
mkunit!(Time);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Distance(pub f32);
mkunit!(Distance);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Speed(pub f32);
mkunit!(Speed);
multiply!(Speed, Time, Distance);
multiply!(Speed, SpcEnergy);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Acceleration(pub f32);
mkunit!(Acceleration);
multiply!(Acceleration, Time, Speed);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SpcEnergy(pub f32);
mkunit!(SpcEnergy);
