#!/usr/bin/env bash
cd $(dirname $0)

# We must stand in this directory to be able to log the URDF because 
# urdf-rerun-loader and `k` handles relative paths differently. 
export ROS_PACKAGE_PATH=$(pwd)/planner:$ROS_PACKAGE_PATH
cd planner/arm_description/urdf/
cargo run --bin play --release