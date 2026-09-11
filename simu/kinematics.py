import math

import numpy as np

WHEEL_ANGLES = np.array(
    [math.pi, math.pi + 2 * math.pi / 3, math.pi + 4 * math.pi / 3]
)


def ik_matrix(wheel_radius, wheel_base):
    sin = np.sin(WHEEL_ANGLES)
    cos = np.cos(WHEEL_ANGLES)
    return np.column_stack([-sin, cos, np.full(3, wheel_base)]) / wheel_radius


def fk_matrix(wheel_radius, wheel_base):
    return np.linalg.inv(ik_matrix(wheel_radius, wheel_base))


def integrate(pose, twist, dt):
    vx, vy, w = twist
    x, y, theta = pose
    pose[0] = x + (vx * math.cos(theta) - vy * math.sin(theta)) * dt
    pose[1] = y + (vx * math.sin(theta) + vy * math.cos(theta)) * dt
    pose[2] = theta + w * dt
