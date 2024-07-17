#!/usr/bin/env bash
cd $(dirname $0)
sed -i \
    -e 's|$(find arm_description)/urdf/||g' \
    -e 's|package://arm_description|..|g' \
    arm_description/urdf/arm.xacro