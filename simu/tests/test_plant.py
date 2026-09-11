import pytest

from controller import ScheduleController
from plant import Plant
from simulator import Simulator


def test_pose_property_returns_copy():
    plant = Plant(1.0, 2.0, 3.0)
    clone = plant.pose
    clone[0] = 99.0
    assert plant.pose[0] == pytest.approx(1.0)


def test_setup_publishes_initial_pose():
    sim = Simulator(dt=0.05)
    sim.add(Plant(1.0, 2.0, 3.0))
    sim.run(0.0)
    assert sim.data.true_pose == pytest.approx([1.0, 2.0, 3.0])


def test_straight_line_from_command():
    sim = Simulator(dt=0.05)
    controller = ScheduleController([(1.0, (0.5, 0.0, 0.0))])
    plant = Plant()
    sim.add(controller)
    sim.add(plant)
    sim.run(controller.end_time)
    assert plant.pose == pytest.approx([0.5, 0.0, 0.0])
