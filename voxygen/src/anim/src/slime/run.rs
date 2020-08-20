use super::{
    super::{vek::*, Animation},
    SlimeSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, Vec3<f32>, Vec3<f32>, f64, Vec3<f32>);
    type Skeleton = SlimeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"slime_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "slime_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, _global_time, avg_vel): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if ::vek::Vec2::new(ori, last_ori)
            .map(|o| o.magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.2)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        };

        let lab = 0.55; //.65

        let short1 = (anim_time as f32 * lab as f32 * 6.0).sin();
        let short2 = (anim_time as f32 * lab as f32 * 6.0 + PI * 0.1).sin();
        let short3 = (anim_time as f32 * lab as f32 * 6.0 + PI * 0.2).sin();
        let short4 = (anim_time as f32 * lab as f32 * 6.0 + PI * 0.3).sin();
        let short5 = (anim_time as f32 * lab as f32 * 6.0 + PI * 0.4).sin();
        
        let short1cos = (anim_time as f32 * lab as f32 * 6.0).cos();
        let short2cos = (anim_time as f32 * lab as f32 * 6.0 + PI * 0.1).cos();
        let short3cos = (anim_time as f32 * lab as f32 * 6.0 + PI * 0.2).cos();

        next.body_1.position = Vec3::new(0.0, (skeleton_attr.body_1.0 + ((short2 * 2.0).abs() - 1.2)) * 10.0, skeleton_attr.body_1.1);
        next.body_1.orientation = Quaternion::rotation_z(0.0);
        next.body_1.scale = Vec3::one() / 4.0 - short5.abs() / 8.0;

        next.body_2.position = Vec3::new(0.0, skeleton_attr.body_2.0, skeleton_attr.body_2.1 + 4.0 - short2.abs());
        next.body_2.orientation = Quaternion::rotation_z(0.0);
        next.body_2.scale = Vec3::one() - ((short4 * 2.0).abs() - 1.1) / 2.0;
        
        next.body_3.position = Vec3::new(0.0, skeleton_attr.body_3.0, skeleton_attr.body_3.1 + 4.0 - short3.abs());
        next.body_3.orientation = Quaternion::rotation_z(0.0);
        next.body_3.scale = Vec3::one() - ((short3 * 3.0).abs() - 1.6) / 2.0;

        next.body_4.position = Vec3::new(0.0, skeleton_attr.body_4.0, skeleton_attr.body_4.1 + 4.0 - short4.abs());
        next.body_4.orientation = Quaternion::rotation_z(0.0);
        next.body_4.scale = Vec3::one() - ((short2 * 4.0).abs() - 2.1) / 2.0;

        next.body_5.position = Vec3::new(0.0, skeleton_attr.body_5.0, skeleton_attr.body_5.1 + 4.0 - short5.abs());
        next.body_5.orientation = Quaternion::rotation_z(0.0);
        next.body_5.scale = Vec3::one() - ((short1 * 5.0).abs() - 2.6) / 2.0;

        next.tail_upper.position = Vec3::new(0.0, skeleton_attr.tail_upper.0 - 12.0 + (short2 * 4.0).abs() , skeleton_attr.tail_upper.1);
        next.tail_upper.orientation = Quaternion::rotation_z(tilt*6.0);
        next.tail_upper.scale = Vec3::one() - ((short3cos * 2.0).abs() - 1.1) / 2.0;

        next.tail_lower.position = Vec3::new(0.0, skeleton_attr.tail_lower.0 - 8.0 + (short3 * 4.0).abs(), skeleton_attr.tail_lower.1);
        next.tail_lower.orientation = Quaternion::rotation_z(tilt*4.0);
        next.tail_lower.scale = Vec3::one() - ((short1cos * 3.0).abs() - 1.6) / 2.0;

        next
    }
}
