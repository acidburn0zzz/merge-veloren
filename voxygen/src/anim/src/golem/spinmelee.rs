use super::{
    super::{vek::*, Animation},
    GolemSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use std::f32::consts::PI;

pub struct SpinMeleeAnimation;

impl Animation for SpinMeleeAnimation {
    type Dependency = (Vec3<f32>, f64, Option<StageSection>);
    type Skeleton = GolemSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"golem_spinmelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "golem_spinmelee")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, _global_time, _stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let lab = 1.0;
        //let speed = Vec2::<f32>::from(velocity).magnitude();
        let mut next = (*skeleton).clone();
        //torso movement
        let xshift = if velocity.z.abs() < 0.1 {
            ((anim_time as f32 - 1.1) * lab as f32 * 3.0).sin()
        } else {
            0.0
        };
        let yshift = if velocity.z.abs() < 0.1 {
            ((anim_time as f32 - 1.1) * lab as f32 * 3.0 + PI / 2.0).sin()
        } else {
            0.0
        };

        let spin = if anim_time < 1.1 && velocity.z.abs() < 0.1 {
            0.5 * ((anim_time as f32).powf(2.0))
        } else {
            lab as f32 * anim_time as f32 * 0.9
        };
        let _movement = anim_time as f32 * 1.0;

        //feet
        //let slowersmooth = (anim_time as f32 * lab as f32 * 4.0).sin();
        //let quick = (anim_time as f32 * lab as f32 * 8.0).sin();

        next.head.position =
            Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1 + 0.2) * 1.02;
        next.head.orientation = Quaternion::rotation_z(0.6) * Quaternion::rotation_x(0.6);
        next.head.scale = Vec3::one();

        next.upper_torso.position = Vec3::new(
            0.0,
            skeleton_attr.upper_torso.0,
            skeleton_attr.upper_torso.1,
        ) / 8.0;
        next.upper_torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.upper_torso.scale = Vec3::one();

        next.lower_torso.position = Vec3::new(
            0.0,
            skeleton_attr.lower_torso.0,
            skeleton_attr.lower_torso.1,
        );
        next.lower_torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.lower_torso.scale = Vec3::one();

        next.shoulder_l.position = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2 - 4.0,
        );
        next.shoulder_l.orientation =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.3);
        next.shoulder_l.scale = Vec3::one();

        next.shoulder_r.position = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2 - 4.0,
        );
        next.shoulder_r.orientation = Quaternion::rotation_z(0.0)
            * Quaternion::rotation_x(0.0)
            * Quaternion::rotation_y(-0.3);
        next.shoulder_r.scale = Vec3::one();

        next.hand_l.position = Vec3::new(
            -skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2,
        );
        next.hand_l.orientation =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.7);
        next.hand_l.scale = Vec3::one();

        next.hand_r.position = Vec3::new(
            skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2,
        );
        next.hand_r.orientation = Quaternion::rotation_z(0.0)
            * Quaternion::rotation_x(0.0)
            * Quaternion::rotation_y(-0.7);
        next.hand_r.scale = Vec3::one();

        next.leg_l.position = Vec3::new(
            -skeleton_attr.leg.0,
            skeleton_attr.leg.1,
            skeleton_attr.leg.2,
        ) * 1.02;
        next.leg_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.leg_l.scale = Vec3::one();

        next.leg_r.position = Vec3::new(
            skeleton_attr.leg.0,
            skeleton_attr.leg.1,
            skeleton_attr.leg.2,
        ) * 1.02;
        next.leg_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.leg_r.scale = Vec3::one();

        next.foot_l.position = Vec3::new(
            -skeleton_attr.foot.0,
            skeleton_attr.foot.1,
            skeleton_attr.foot.2 + -0.2,
        );
        next.foot_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.foot_l.scale = Vec3::one();

        next.foot_r.position = Vec3::new(
            skeleton_attr.foot.0,
            skeleton_attr.foot.1,
            skeleton_attr.foot.2 + -0.2,
        );
        next.foot_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.foot_r.scale = Vec3::one();
        //next.shoulder_l.position = Vec3::new(
        //    -skeleton_attr.shoulder.0,
        //    skeleton_attr.shoulder.1,
        //    skeleton_attr.shoulder.2,
        //);
        //next.shoulder_l.orientation = Quaternion::rotation_z(0.07)
        //    * Quaternion::rotation_y(0.15)
        //    * Quaternion::rotation_x(-0.25);
        //next.shoulder_l.scale = Vec3::one();

        //next.shoulder_r.position = Vec3::new(
        //    skeleton_attr.shoulder.0,
        //    skeleton_attr.shoulder.1,
        //    skeleton_attr.shoulder.2,
        //);
        //next.shoulder_r.orientation = Quaternion::rotation_z(-0.07)
        //    * Quaternion::rotation_y(-0.15)
        //    * Quaternion::rotation_x(-0.25);
        //next.shoulder_r.scale = Vec3::one();

        ////next.hand_l.position = Vec3::new(-0.75, -1.0, 2.5);
        //next.hand_l.position = Vec3::new(
        //    -skeleton_attr.hand.0,
        //    skeleton_attr.hand.1,
        //    skeleton_attr.hand.2,
        //);
        //next.hand_l.orientation = Quaternion::rotation_x(1.47) *
        // Quaternion::rotation_y(-0.2); next.hand_l.scale = Vec3::one() * 1.05;

        ////next.hand_r.position = Vec3::new(0.75, -1.5, -0.5);
        //next.hand_r.position = Vec3::new(
        //    skeleton_attr.hand.0,
        //    skeleton_attr.hand.1,
        //    skeleton_attr.hand.2,
        //);
        //next.hand_r.orientation = Quaternion::rotation_x(1.47) *
        // Quaternion::rotation_y(0.3); next.hand_r.scale = Vec3::one() * 1.05;
        ////next.main.position = Vec3::new(0.0, 0.0, 2.0);
        ////next.main.orientation = Quaternion::rotation_x(-0.1)
        ////
        ////    * Quaternion::rotation_y(0.0)
        ////    * Quaternion::rotation_z(0.0);
        //next.head.position = Vec3::new(0.0, skeleton_attr.head.0 + 0.0,
        // skeleton_attr.head.1);

        //next.hand_l.position = Vec3::new(-0.5, 0.0, 4.0);
        //next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
        //    * Quaternion::rotation_z(0.0)
        //    * Quaternion::rotation_y(PI);
        //next.hand_l.scale = Vec3::one() * 1.08;
        //next.hand_r.position = Vec3::new(0.5, 0.0, -2.5);
        //next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
        //    * Quaternion::rotation_z(0.0)
        //    * Quaternion::rotation_y(0.0);
        //next.hand_r.scale = Vec3::one() * 1.06;
        ////next.main.position = Vec3::new(-0.0, -2.0, -1.0);
        ////next.main.orientation = Quaternion::rotation_x(0.0)
        ////
        ////    * Quaternion::rotation_y(0.0)
        ////    * Quaternion::rotation_z(0.0);

        ////next.control.position = Vec3::new(0.0, 16.0, 3.0);
        ////next.control.orientation = Quaternion::rotation_x(-1.4)
        ////
        ////    * Quaternion::rotation_y(0.0)
        ////    * Quaternion::rotation_z(1.4);
        ////next.control.scale = Vec3::one();

        //next.head.position = Vec3::new(0.0, skeleton_attr.head.0,
        // skeleton_attr.head.1); next.head.orientation =
        // Quaternion::rotation_z(0.0)
        //    * Quaternion::rotation_x(-0.15)
        //    * Quaternion::rotation_y(0.08);
        //next.upper_torso.position = Vec3::new(
        //    0.0,
        //    skeleton_attr.upper_torso.0 - 3.0,
        //    skeleton_attr.upper_torso.1 - 2.0,
        //);
        //next.upper_torso.orientation = Quaternion::rotation_z(0.0)
        //    * Quaternion::rotation_x(-0.1)
        //    * Quaternion::rotation_y(0.3);
        //next.upper_torso.scale = Vec3::one();

        ////next.belt.position = Vec3::new(0.0, 1.0, -1.0);
        ////next.belt.orientation = Quaternion::rotation_z(0.0)
        ////
        ////    * Quaternion::rotation_x(0.4)
        ////    * Quaternion::rotation_y(0.0);
        ////next.belt.scale = Vec3::one() * 0.98;
        //next.leg_l.position = Vec3::new(0.0, 3.0, -2.5);
        //next.leg_l.orientation =
        //    Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.7) *
        // Quaternion::rotation_y(0.0); next.leg_l.scale = Vec3::one();
        //next.leg_r.position = Vec3::new(0.0, 3.0, -2.5);
        //next.leg_r.orientation =
        //    Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.7) *
        // Quaternion::rotation_y(0.0); next.leg_r.scale = Vec3::one();
        next.torso.position = Vec3::new(
            -xshift * (anim_time as f32).min(0.6),
            -yshift * (anim_time as f32).min(0.6),
            4.0,
        );
        next.torso.orientation = Quaternion::rotation_z(spin * -16.0)
            * Quaternion::rotation_x(0.0)
            * Quaternion::rotation_y(0.0);
        next.torso.scale = Vec3::one() / 8.0;
        //if velocity.z.abs() > 0.1 {
        //    next.foot_l.position =
        //        Vec3::new(-skeleton_attr.foot.0, 8.0, skeleton_attr.foot.2 + 2.0);
        //    next.foot_l.orientation = Quaternion::rotation_x(1.0) *
        // Quaternion::rotation_z(0.0);    next.foot_l.scale = Vec3::one();

        //    next.foot_r.position = Vec3::new(skeleton_attr.foot.0, 8.0,
        // skeleton_attr.foot.2 + 2.0);    next.foot_r.orientation =
        // Quaternion::rotation_x(1.0);    next.foot_r.scale = Vec3::one();
        //} else if speed < 0.5 {
        //    next.foot_l.position = Vec3::new(
        //        -skeleton_attr.foot.0,
        //        2.0 + quick * -6.0,
        //        skeleton_attr.foot.2,
        //    );
        //    next.foot_l.orientation =
        //        Quaternion::rotation_x(0.5 + slowersmooth * 0.2) *
        // Quaternion::rotation_z(0.0);    next.foot_l.scale = Vec3::one();

        //    next.foot_r.position = Vec3::new(skeleton_attr.foot.0, 4.0,
        // skeleton_attr.foot.2);    next.foot_r.orientation =
        //        Quaternion::rotation_x(0.5 - slowersmooth * 0.2) *
        // Quaternion::rotation_y(-0.4);    next.foot_r.scale = Vec3::one();
        //} else {
        //    next.foot_l.position = Vec3::new(
        //        -skeleton_attr.foot.0,
        //        2.0 + quick * -6.0,
        //        skeleton_attr.foot.2,
        //    );
        //    next.foot_l.orientation =
        //        Quaternion::rotation_x(0.5 + slowersmooth * 0.2) *
        // Quaternion::rotation_z(0.0);    next.foot_l.scale = Vec3::one();

        //    next.foot_r.position = Vec3::new(
        //        skeleton_attr.foot.0,
        //        2.0 + quick * 6.0,
        //        skeleton_attr.foot.2,
        //    );
        //    next.foot_r.orientation =
        //        Quaternion::rotation_x(0.5 - slowersmooth * 0.2) *
        // Quaternion::rotation_z(0.0);    next.foot_r.scale = Vec3::one();
        //}

        ////next.lantern.position = Vec3::new(
        ////    skeleton_attr.lantern.0,
        ////    skeleton_attr.lantern.1,
        ////    skeleton_attr.lantern.2,
        ////);
        ////next.lantern.orientation = Quaternion::rotation_z(0.0)
        ////    * Quaternion::rotation_x(0.7)
        ////    * Quaternion::rotation_y(-0.8);
        ////next.hold.scale = Vec3::one() * 0.0;
        ////next.glider.position = Vec3::new(0.0, 0.0, 10.0);
        ////next.glider.scale = Vec3::one() * 0.0;
        ////next.l_control.scale = Vec3::one();
        ////next.r_control.scale = Vec3::one();

        ////next.second.scale = match (
        ////    active_tool_kind.map(|tk| tk.hands()),
        ////    second_tool_kind.map(|tk| tk.hands()),
        ////) {
        ////    (Some(Hands::OneHand), Some(Hands::OneHand)) => Vec3::one(),
        ////    (_, _) => Vec3::zero(),
        ////};

        next
    }
}
