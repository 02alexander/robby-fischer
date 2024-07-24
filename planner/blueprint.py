#!/usr/bin/env python3

import rerun as rr
import rerun.blueprint as rrb
import argparse

space_view_defaults = [
    rr.components.AxisLength(0.0), # To hide the axises of all the transformations.
    rr.components.ImagePlaneDistance(0.3),
    # rr.components.Radius(0.2),
]

blueprint = rrb.Blueprint(
    rrb.Horizontal(
        rrb.Vertical(
            rrb.Spatial2DView(
                origin="a8origin/pinhole/image"
            ),
            rrb.Spatial2DView(
                contents=[
                    "images/mask",
                    "images/points",
                ]
            ),
        ),
        rrb.Vertical(
            rrb.Spatial2DView(
                origin="external_camera",
            ),
            # View that follows the claw
            rrb.Spatial3DView(
                origin="/arm.urdf/base_link/glid_platta_1/bas_1/gemensam_vagg_1/botten_snurr_1/kortarm_kopia_1/led_1/led_axel_1/lang_arm_1/mount_1/ram_1", #
                contents="/**",
                defaults=space_view_defaults
            )
        ),
        rrb.Spatial3DView(
            defaults=space_view_defaults
        ),
        column_shares=[2,2,3]
    ),
    auto_space_views=False,
    collapse_panels=True,
)

parser = argparse.ArgumentParser()
parser.add_argument("--recording-id", type=str)
parser.add_argument("--application-id", type=str)

args = parser.parse_args()
rr.init(args.application_id, recording_id=args.recording_id)
rr.connect()
rr.send_blueprint(blueprint)

