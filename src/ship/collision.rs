use super::ActorNative;
use super::units;
use nalgebra::{Vector2, Matrix2};

pub fn reflect(left: &mut ActorNative, right: &mut ActorNative, normal: Vector2<f32>, angularleft: units::TrueSpaceUnit<f32>, angularright: units::TrueSpaceUnit<f32>) {
    // normal vector need not be normalized, and sign does not matter.
    // angular values are multiplied by force to derive torque:
    // already account for both distance and angle.
    const ELASTICITY: f32 = 0.5;
    
    // vector and dimensional analysis libraries are not compatible
    let leftvelocitystart = Vector2::new(left.dx.value_unsafe, left.dy.value_unsafe);
    let rightvelocitystart = Vector2::new(right.dx.value_unsafe, right.dy.value_unsafe);
    let leftmomentum = leftvelocitystart * left.specs.mass.value_unsafe;
    let rightmomentum = rightvelocitystart * right.specs.mass.value_unsafe;
    
    let centervelocity: Vector2<f32> = (leftmomentum + rightmomentum) / (left.specs.mass.value_unsafe + right.specs.mass.value_unsafe);// reference frame to make reflections simpler

    let leftvelocity = leftvelocitystart - centervelocity;
    let rightvelocity = rightvelocitystart - centervelocity;

    // rotation matrix for normal, plus a scale factor that will be compensated for later
    let toaxis = Matrix2::new(
	normal.x, normal.y,
	-normal.y,normal.x,
    );
    let fromaxis = Matrix2::new(
	normal.x, -normal.y,
	normal.y,normal.x,
    );

    let leftvelocity: Vector2<f32> = toaxis * leftvelocity;
    let rightvelocity: Vector2<f32> = toaxis * rightvelocity;

    let leftvelocity = Vector2::new(-leftvelocity.x * ELASTICITY, leftvelocity.y);
    let rightvelocity = Vector2::new(-rightvelocity.x * ELASTICITY, rightvelocity.y);

    // Rotation part is reversed
    // Magnitude is applied again, resulting in a squared value
    let leftvelocity: Vector2<f32> = fromaxis * leftvelocity;
    let rightvelocity: Vector2<f32> = fromaxis * rightvelocity;

    let normal_magnitude_inv_sq = 1.0/(normal.x*normal.x + normal.y*normal.y);

    let leftvelocity = leftvelocity * normal_magnitude_inv_sq;
    let rightvelocity = rightvelocity * normal_magnitude_inv_sq;
    
    let leftvelocity = leftvelocity + centervelocity;
    let rightvelocity = rightvelocity + centervelocity;

    (left.dx.value_unsafe, left.dy.value_unsafe) = (leftvelocity.x, leftvelocity.y);
    (right.dx.value_unsafe, right.dy.value_unsafe) = (rightvelocity.x, rightvelocity.y);

    let leftdeltav = units::TrueSpaceUnitPerSecond::new((leftvelocitystart - leftvelocity).norm());
    let rightdeltav = units::TrueSpaceUnitPerSecond::new((rightvelocitystart - rightvelocity).norm());

    let leftdeltaomega = (leftdeltav * angularleft) / left.specs.inertia;
    let rightdeltaomega = (rightdeltav * angularright) / right.specs.inertia;

    left.angularvelocity += leftdeltaomega;
    right.angularvelocity += rightdeltaomega;
}
