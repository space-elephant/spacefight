use super::ActorNative;

pub fn reflect(left: &mut ActorNative, right: &mut ActorNative) {
    // TODO
    left.dx = -left.dx * 0.5;
    left.dy = -left.dy * 0.5;
    right.dx = -right.dx * 0.5;
    right.dy = -right.dy * 0.5;
}
