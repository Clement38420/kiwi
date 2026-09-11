import numpy as np

from kinematics import integrate
from module import PRIORITY_PLANT, Module


class Plant(Module):
    priority = PRIORITY_PLANT

    def __init__(self, x=0.0, y=0.0, theta=0.0):
        self._pose = np.array([x, y, theta], dtype=float)

    @property
    def pose(self):
        return self._pose.copy()

    def setup(self, sim):
        sim.data.true_pose = self._pose.copy()

    def step(self, sim):
        assert sim.data.command is not None
        integrate(self._pose, sim.data.command, sim.dt)
        sim.data.true_pose = self._pose.copy()
