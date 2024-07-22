use std::collections::HashMap;

use rand::distributions::{Distribution, Uniform};
use rerun::external::glam::Vec3;
use rrt::smooth_path;

use crate::{
    board::Board,
    chess::{Piece, Square},
    visualizer::{board_to_real_cord, BoundingBox, PieceModelInfo},
};

fn is_free(
    pos: Vec3,
    mut bounding_box: BoundingBox,
    board: &Board,
    piece_size_info: &HashMap<Piece, PieceModelInfo>,
) -> bool {
    bounding_box.center += pos;
    for file in 0..8 {
        for rank in 0..8 {
            if let Some(piece) = board.position[file][rank] {
                let mut piece_bb = piece_size_info.get(&piece).unwrap().bounding_box;
                piece_bb.center =
                    piece_bb.center * 0.001 + board_to_real_cord(Square::new(file, rank));
                piece_bb.half_size = piece_bb.half_size * 0.001;
                if bounding_box.intersects(&piece_bb) {
                    return false;
                }
            }
        }
    }
    true
}

pub fn find_path(
    start: Vec3,
    end: Vec3,
    bounding_box: BoundingBox,
    board: &Board,
    piece_size_info: &HashMap<Piece, PieceModelInfo>,
) -> anyhow::Result<Vec<Vec3>> {
    dbg!(start);
    dbg!(end);
    let mut path = rrt::dual_rrt_connect(
        &[start[0], start[1], start[2]],
        &[end[0], end[1], end[2]],
        |p| {
            is_free(
                Vec3::new(p[0], p[1], p[2]),
                bounding_box,
                board,
                piece_size_info,
            )
        },
        || {
            let between = Uniform::new(-0.01, 0.2);
            let zrange = Uniform::new(0.001, 0.3);
            let mut rng = rand::thread_rng();
            vec![
                between.sample(&mut rng),
                between.sample(&mut rng),
                zrange.sample(&mut rng),
            ]
        },
        0.01,
        1000,
    )
    .map_err(|e| anyhow::anyhow!(e))?;

    smooth_path(
        &mut path,
        |p| {
            is_free(
                Vec3::new(p[0], p[1], p[2]),
                bounding_box,
                board,
                piece_size_info,
            )
        },
        0.001,
        10,
    );

    Ok(path.iter().map(|p| Vec3::new(p[0], p[1], p[2])).collect())
}
