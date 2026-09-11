import numpy as np

from kinematics import fk_matrix, ik_matrix, integrate
from module import PRIORITY_SENSOR, Module


class WheelOdometry(Module):
    priority = PRIORITY_SENSOR

    def __init__(
        self,
        wheel_radius=0.068,
        wheel_base=0.16,
        wheel_speed_std=0.2,
        slip=0.9,
        x=0.0,
        y=0.0,
        theta=0.0,
    ):
        self.ik = ik_matrix(wheel_radius, wheel_base)
        self.fk = fk_matrix(wheel_radius, wheel_base)
        self.wheel_speed_std = wheel_speed_std
        self.slip = slip
        self.rng = np.random.default_rng()
        self._pose = np.array([x, y, theta], dtype=float)

    @property
    def pose(self):
        return self._pose.copy()

    def setup(self, sim):
        sim.data.odom_twist = np.zeros(3)
        sim.data.odom_pose = self._pose.copy()

    def step(self, sim):
        assert sim.data.command is not None
        wheels = self.ik @ sim.data.command
        measured = wheels * self.slip + self.rng.normal(
            0.0, self.wheel_speed_std, 3
        )
        twist = self.fk @ measured
        integrate(self._pose, twist, sim.dt)
        sim.data.odom_twist = twist
        sim.data.odom_pose = self._pose.copy()
