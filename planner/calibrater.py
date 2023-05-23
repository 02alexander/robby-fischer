#!/usr/bin/env python3

import jax.numpy as jnp
from jax import grad, jit

def error(v):
    
    trans = v[0:3]
    theta1_offset, theta2_offset = v[3:]
    pairs = [
        (jnp.array([0.9773843811168232, 2.0385445663293718, 0.09000000000000007]), jnp.array([0, 0.0, 0.0])),
        (jnp.array([1.0611601852125507, 1.8570303241219621, 0.09000000000000007]), jnp.array([0.05, 0.0, 0.0])),
        (jnp.array([1.1588986233242329, 1.6755160819145525, 0.09000000000000007]), jnp.array([0.1, 0.0, 0.0])),
        (jnp.array([1.2426744274199601, 1.466076571675234, 0.09000000000000007]), jnp.array([0.15000000000000002, 0.0, 0.0])),
        (jnp.array([1.3404128655316423, 1.256637061435915, 0.09000000000000007]), jnp.array([0.2, 0.0, 0.0])),
        (jnp.array([1.4521139376592793, 1.019272283164687, 0.09000000000000007]), jnp.array([0.25, 0.0, 0.0])),
        (jnp.array([1.5777776438028703, 0.75398223686155, 0.09000000000000007]), jnp.array([0.30000000000000004, 0.0, 0.0])),
        (jnp.array([1.7034413499464618, 0.4607669225265024, 0.09000000000000007]), jnp.array([0.35000000000000003, 0.0, 0.0])),
    ]
    
    arm1 = jnp.array([-0.29, 0])
    arm2 = jnp.array([-0.29, 0])

    sm = 0
    for (angles, cord) in pairs:
        theta1, theta2 = angles[:2]
        theta1 = theta1 + theta1_offset
        theta2 = theta2 + theta2_offset
        rot_mat1 = jnp.array([
            [jnp.cos(-theta1), -jnp.sin(-theta1)],
            [jnp.sin(-theta1), jnp.cos(-theta1)]
        ])
        rot_mat2 = jnp.array([
            [jnp.cos(-theta2), -jnp.sin(-theta2)],
            [jnp.sin(-theta2), jnp.cos(-theta2)]
        ])
        claw_pos = (rot_mat1 @ (arm1 + (rot_mat2  @ arm2))) + trans[:2]
        sm += (jnp.linalg.norm(claw_pos - cord[:2]))**2
    
    return sm #+ 0.01*jnp.linalg.norm(trans[:2]) + 0.1*jnp.linalg.norm(v[3:])

def gd(grad, x0, n, alpha, momentum=0.9): 
    acc = jnp.zeros_like(grad(x0))
    for _ in range(n):
        acc = grad(x0) + momentum*acc
        x0 += -acc*alpha
    return x0

def claw_pos(angles):
    arm1 = jnp.array([-0.29, 0])
    arm2 = jnp.array([-0.29, 0])

    theta1, theta2 = angles
    rot_mat1 = jnp.array([
        [jnp.cos(-theta1), -jnp.sin(-theta1)],
        [jnp.sin(-theta1), jnp.cos(-theta1)]
    ])
    rot_mat2 = jnp.array([
        [jnp.cos(-theta2), -jnp.sin(-theta2)],
        [jnp.sin(-theta2), jnp.cos(-theta2)]
    ])
    claw_pos = rot_mat1 @ (arm1 + (rot_mat2  @ arm2))
    return claw_pos

print()
print(error(jnp.array([0.0, 0.0, 0.0, 0.0, 0.0])))

effgrad = jit(grad(error)) 

params = gd(effgrad, 0.0001*jnp.ones((5,)), 1000, 0.01, 0.95)

print(error(params))
print(f'translation = {params[:3]}')
print(f'angle offsets = {180/jnp.pi*params[3:]}')

