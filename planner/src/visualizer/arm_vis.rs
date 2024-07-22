use std::sync::Mutex;

use once_cell::sync::Lazy;
use rerun::{Angle, Quaternion, Rotation3D, RotationAxisAngle, Vec3D};

use crate::{chess::Piece, utils::MyIntersperseExt};

pub const URDF_PATH: &str = "arm.urdf";

pub static CHAIN: Lazy<Mutex<k::Chain<f32>>> = Lazy::new(|| {
    let chain = k::Chain::<f32>::from_urdf_file(URDF_PATH).unwrap();
    Mutex::new(chain)
});

fn get_entity_path(link: &k::Node<f32>) -> String {
    let mut ancestors: Vec<_> = link
        .iter_ancestors()
        .map(|node| node.link().as_ref().unwrap().name.clone())
        .collect();
    ancestors.push(String::from(URDF_PATH));
    ancestors
        .into_iter()
        .rev()
        .my_intersperse(String::from("/"))
        .collect()
}

pub fn init_arm_vis(rec: &rerun::RecordingStream) {
    rec.log_file_from_path(URDF_PATH, None, false).unwrap();
    rec.log(
        "arm.urdf",
        &rerun::Transform3D::from_translation_rotation(
            [-0.185, 0.080, 0.04],
            Rotation3D::AxisAngle(RotationAxisAngle::new([0., 0., 1.], Angle::Degrees(180.0))),
        ),
    )
    .unwrap();
}

pub fn log_robot_state(
    sideways_m: f32,
    bottom_deg: f32,
    top_deg: f32,
    claw_state: Option<Piece>,
) -> Option<()> {
    // let rec = REC.lock().unwrap();
    let rec = rerun::RecordingStream::thread_local(rerun::StoreKind::Recording)?;
    let chain = CHAIN.lock().unwrap();
    let bottom = bottom_deg.to_radians();
    let top = top_deg.to_radians();

    let mut positions = chain.joint_positions();
    positions[0] = -(sideways_m - 0.2);
    positions[1] = -(bottom - std::f32::consts::PI / 2.0);
    positions[2] = -(top - std::f32::consts::PI / 2.0);
    positions[3] = -(bottom + top - std::f32::consts::PI);
    if claw_state.is_some() {
        positions[4] = -0.02;
        positions[5] = -0.02;
    } else {
        positions[4] = 0.0;
        positions[5] = 0.0;
    }
    chain.set_joint_positions(&positions).unwrap();

    chain.update_transforms();
    chain.update_link_transforms();

    for link_name in chain.iter_links().map(|link| link.name.clone()) {
        let link = chain.find_link(&link_name).unwrap();
        let entity_path = get_entity_path(&link);
        let link_to_world = link.world_transform().unwrap();
        let link_to_parent = if link_name != "base_link" {
            let parent = link.parent().unwrap();
            parent.world_transform().unwrap().inv_mul(&link_to_world)
        } else {
            link_to_world
        };
        let link_to_parent_mat = link_to_parent.to_matrix();

        let trans = link_to_parent_mat.column(3);
        let trans = trans.as_slice();
        let quat = link_to_parent.rotation.quaternion();
        let _rot = Rotation3D::Quaternion(Quaternion(quat.coords.as_slice().try_into().unwrap()));
        rec.log(
            entity_path,
            &rerun::Transform3D::from_translation_rotation(
                Vec3D::new(trans[0], trans[1], trans[2]),
                Rotation3D::Quaternion(Quaternion(quat.coords.as_slice().try_into().unwrap())),
            ),
        )
        .unwrap();
    }
    Some(())
}
