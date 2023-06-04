#!/usr/bin/env python3

import jax.numpy as jnp
from jax import grad, jit

def error(v):
    
    trans = v[0:3]
    theta1_offset, theta2_offset = v[3:]
    pairs = [
        # (jnp.array([0.9773843811168232, 2.0385445663293718, 0.09000000000000007]), jnp.array([0, 0.0, 0.0])),
        # (jnp.array([1.0611601852125507, 1.8430676901060072, 0.09000000000000007]), jnp.array([0.05, 0.0, 0.0])),

        # (jnp.array([0.9773843811168232, 2.0385445663293718, 0.09000000000000007]), jnp.array([0, 0.0, 0.0])),
        # (jnp.array([1.0611601852125507, 1.8570303241219621, 0.09000000000000007]), jnp.array([0.05, 0.0, 0.0])),
        # (jnp.array([1.1588986233242329, 1.6755160819145525, 0.09000000000000007]), jnp.array([0.1, 0.0, 0.0])),
        # (jnp.array([1.2426744274199601, 1.466076571675234, 0.09000000000000007]), jnp.array([0.15000000000000002, 0.0, 0.0])),
        # (jnp.array([1.3404128655316423, 1.256637061435915, 0.09000000000000007]), jnp.array([0.2, 0.0, 0.0])),
        # (jnp.array([1.4521139376592793, 1.019272283164687, 0.09000000000000007]), jnp.array([0.25, 0.0, 0.0])),
        # (jnp.array([1.5777776438028703, 0.75398223686155, 0.09000000000000007]), jnp.array([0.30000000000000004, 0.0, 0.0])),
        # (jnp.array([1.7034413499464618, 0.4607669225265024, 0.09000000000000007]), jnp.array([0.35000000000000003, 0.0, 0.0])),

#         (jnp.array([0.9997485693184842, 2.022509566879566, 0.09200000000000007]), jnp.array([0, 0.0, 0.0])),
# (jnp.array([1.0974870074301664, 1.8270326906562018, 0.09200000000000007]), jnp.array([0.05, 0.0, 0.0])),
# (jnp.array([1.1952254455418487, 1.6315558144328375, 0.09200000000000007]), jnp.array([0.1, 0.0, 0.0])),
# (jnp.array([1.2929638836535307, 1.4360789382094734, 0.09200000000000007]), jnp.array([0.15000000000000002, 0.0, 0.0])),
# (jnp.array([1.3907023217652126, 1.1987141599382456, 0.09200000000000007]), jnp.array([0.2, 0.0, 0.0])),
# (jnp.array([1.5024033938928496, 0.9753120156829717, 0.09200000000000007]), jnp.array([0.25, 0.0, 0.0])),
# (jnp.array([1.6141044660204862, 0.710021969379835, 0.09200000000000007]), jnp.array([0.30000000000000004, 0.0, 0.0])),
# (jnp.array([1.725805538148122, 0.43076928906074397, 0.09200000000000007]), jnp.array([0.35000000000000003, 0.0, 0.0])),

#         (jnp.array([0.90757121103705, 2.0385445663293718, 0.2800000000000002]), jnp.array([0, 0.0, 0.0])),
# (jnp.array([1.0192722831646868, 1.8570303241219623, 0.2800000000000002]), jnp.array([0.05, 0.0, 0.0])),
# (jnp.array([1.117010721276369, 1.6615534478985983, 0.2800000000000002]), jnp.array([0.1, 0.0, 0.0])),
# (jnp.array([1.2147491593880515, 1.452113937659279, 0.2800000000000002]), jnp.array([0.15000000000000002, 0.0, 0.0])),
# (jnp.array([1.3264502315156879, 1.228711793404006, 0.2800000000000002]), jnp.array([0.2, 0.0, 0.0])),
# (jnp.array([1.42418866962737, 1.019272283164687, 0.2800000000000002]), jnp.array([0.25, 0.0, 0.0])),
# (jnp.array([1.549852375770961, 0.7400196028455958, 0.2800000000000002]), jnp.array([0.30000000000000004, 0.0, 0.0])),
# (jnp.array([1.6894787159305062, 0.43284165449459605, 0.2800000000000002]), jnp.array([0.35000000000000003, 0.0, 0.0])),

    (jnp.array([0.9175126715440673, 2.018820236558258, 0.2799900472164154]), jnp.array([0, 0.0, 0.0])),
(jnp.array([1.0431763776876584, 1.8233433603348945, 0.2799900472164154]), jnp.array([0.05, 0.0, 0.0])),
(jnp.array([1.14091481579934, 1.6418291181274849, 0.2799900472164154]), jnp.array([0.1, 0.0, 0.0])),
(jnp.array([1.2386532539110227, 1.4323896078881657, 0.2799900472164154]), jnp.array([0.15000000000000002, 0.0, 0.0])),
(jnp.array([1.3363916920227048, 1.222950097648847, 0.2799900472164154]), jnp.array([0.2, 0.0, 0.0])),
(jnp.array([1.4480927641503416, 0.9855853193776187, 0.2799900472164154]), jnp.array([0.25, 0.0, 0.0])),
(jnp.array([1.5737564702939335, 0.7342579070904354, 0.2799900472164154]), jnp.array([0.30000000000000004, 0.0, 0.0])),
(jnp.array([1.7133828104534794, 0.4550052267713429, 0.2799900472164154]), jnp.array([0.35000000000000003, 0.0, 0.0])),

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
        sm += (jnp.linalg.norm(claw_pos - cord[:2]))**2 + (angles[2] + trans[2])**2
    
    return sm + 0.01*jnp.linalg.norm(trans[:2]) # + 0.1*jnp.linalg.norm(v[3:])

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

trans = params[:3]
rx = trans[0]
ry = trans[2]
rz = trans[1] 
angles = params[3:]
print(error(params))
print(f'    arm.bottom_angle_offset = {180/jnp.pi*params[3]};')
print(f'    arm.top_angle_offset = {180/jnp.pi*params[4]};')
# print([float(e) for e in params[:3]])
print(f'    arm.translation_offset = Vector3::new({", ".join([str(float(e)) for e in [rx, ry, rz]]) });')

# print(f'translation = {params[:3]}')
# print(f'angle offsets = {180/jnp.pi*params[3:]}')

