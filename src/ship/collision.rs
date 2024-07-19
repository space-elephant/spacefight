use super::ActorNative;
use super::units;
use nalgebra::{Vector2, Matrix2};

pub fn reflect(left: &mut ActorNative, right: &mut ActorNative, normal: Vector2<f32>) {
    // normal vector need not be normalized, and sign does not matter.
    const ELASTICITY: f32 = 0.5;
    
    // vector and dimensional analysis libraries are not compatible
    let leftvelocity = Vector2::new(left.dx.value_unsafe, left.dy.value_unsafe);
    let rightvelocity = Vector2::new(right.dx.value_unsafe, right.dy.value_unsafe);
    let leftmomentum = leftvelocity * left.specs.mass.value_unsafe;
    let rightmomentum = rightvelocity * right.specs.mass.value_unsafe;
    
    let centervelocity: Vector2<f32> = (leftmomentum + rightmomentum) / (left.specs.mass.value_unsafe + right.specs.mass.value_unsafe);// reference frame to make reflections simpler

    let leftvelocity = leftvelocity - centervelocity;
    let rightvelocity = rightvelocity - centervelocity;

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
}
