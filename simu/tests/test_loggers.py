import pytest

from controller import ScheduleController
from loggers import PoseLogger
from plant import Plant
from simulator import Simulator


def test_skips_missing_field():
    sim = Simulator(dt=0.1)
    logger = PoseLogger("odom_pose")
    sim.add(logger)
    sim.run(0.2)
    assert logger.history == []


def test_records_pose_each_tick():
    sim = Simulator(dt=0.1)
    sim.add(ScheduleController([(0.2, (0.0, 0.0, 0.0))]))
    sim.add(Plant())
    logger = PoseLogger("true_pose")
    sim.add(logger)
    sim.run(0.2)
    assert len(logger.history) == 3
    assert [t for t, _ in logger.history] == pytest.approx([0.0, 0.1, 0.2])
