import numpy as np
import pytest

from controller import ScheduleController
from plant import Plant
from simulator import Simulator
from wheel_odometry import WheelOdometry


def _run(commands, odom_kwargs=None, dt=0.05):
    sim = Simulator(dt=dt)
    controller = ScheduleController(commands)
    plant = Plant()
    odom = WheelOdometry(**(odom_kwargs or {}))
    sim.add(controller)
    sim.add(plant)
    sim.add(odom)
    sim.run(controller.end_time)
    return plant, odom, sim


def test_noise_free_odom_matches_true_pose():
    plant, odom, _ = _run(
        [(1.0, (0.5, 0.1, 0.3))], {"wheel_speed_std": 0.0, "slip": 1.0}
    )
    assert odom.pose == pytest.approx(plant.pose)


def test_publishes_blackboard_fields():
    _, _, sim = _run([(0.1, (1.0, 0.0, 0.0))])
    assert sim.data.odom_twist is not None
    assert sim.data.odom_pose is not None


def test_noise_causes_drift():
    plant, odom, _ = _run([(2.0, (0.5, 0.2, 0.1))], {"wheel_speed_std": 0.05})
    assert not np.allclose(odom.pose, plant.pose)
