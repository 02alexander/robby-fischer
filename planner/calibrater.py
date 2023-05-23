#!/usr/bin/env python3

import jax.numpy as jnp
from jax import grad, jit

def error(v):
    
    trans = v[0:3]
    theta1_offset, theta2_offset = v[3:]
    pairs = [
        (jnp.array([1.0109758887302411, 2.1296913808343376]), jnp.array([0.0, 0.0])),
        (jnp.array([1.2981282420338651, 1.532570609166752]), jnp.array([0.15, 0.0])),
        (jnp.array([1.7646371102937313, 0.539613651003943]), jnp.array([0.35, 0.0])),

        # (jnp.array([0.808379638593022, 0.9227392615206563]), jnp.array([0.29+0.1, 0.29]))
    ]
    
    arm1 = jnp.array([-0.29, 0])
    arm2 = jnp.array([-0.29, 0])

    sm = 0
    for (angles, cord) in pairs:
        theta1, theta2 = angles
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
        sm += (jnp.linalg.norm(claw_pos - cord))**2
    
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

# print(claw_pos(jnp.array([1.01 - (jnp.pi/180.0)*38, 2.1 + (jnp.pi/180)*25])))
# print(180/jnp.pi * jnp.array([1.01 - (jnp.pi/180.0)*43, 2.1 - (jnp.pi/180)*43]))
# print(180/jnp.pi * jnp.array([1.01 - (jnp.pi/180.0)*43, 2.1 - (jnp.pi/180)*43]))

# print(claw_pos(jnp.array([1.01 - (jnp.pi/180.0)*43, 2.1 - (jnp.pi/180)*43])))
# print(claw_pos(jnp.array([1.01 - (jnp.pi/180.0)*38, 2.1 + (jnp.pi/180)*25])))
# print(claw_pos(jnp.array([1.01, 2.1])))
# print(claw_pos(jnp.array([1.57, 1.58])))

print()
print(error(jnp.array([0.0, 0.0, 0.0, 0.0, 0.0])))

effgrad = jit(grad(error)) 

params = gd(effgrad, 0.0001*jnp.ones((5,)), 1000, 0.01, 0.95)

print(error(params))
print(f'translation = {params[:3]}')
print(f'angle offsets = {180/jnp.pi*params[3:]}')

