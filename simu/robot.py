import math

import numpy as np


class Robot:
    def __init__(self, x=0.0, y=0.0, theta=0.0):
        self.pose = np.array([x, y, theta], dtype=float)
        self.twist = np.zeros(3)
        self.history = [self.pose.copy()]

    def command(self, vx, vy, w):
        self.twist[:] = (vx, vy, w)

    def step(self, dt):
        vx, vy, w = self.twist
        x, y, theta = self.pose
        self.pose[0] = x + (vx * math.cos(theta) - vy * math.sin(theta)) * dt
        self.pose[1] = y + (vx * math.sin(theta) + vy * math.cos(theta)) * dt
        self.pose[2] = theta + w * dt
        self.history.append(self.pose.copy())
