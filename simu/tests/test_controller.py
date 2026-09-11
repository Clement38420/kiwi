import pytest

from controller import ScheduleController
from simulator import Simulator


def test_end_time_sums_durations():
    controller = ScheduleController([(1.0, (0, 0, 0)), (2.0, (1, 0, 0))])
    assert controller.end_time == pytest.approx(3.0)


def test_command_changes_at_segment_boundary():
    controller = ScheduleController([(0.1, (1, 0, 0)), (0.1, (0, 1, 0))])
    sim = Simulator(dt=0.1)
    sim.add(controller)
    sim.run(controller.end_time)
    assert controller.command == pytest.approx([0.0, 1.0, 0.0])


def test_set_command_overrides_schedule():
    controller = ScheduleController([(1.0, (1, 0, 0))])
    sim = Simulator(dt=0.05)
    sim.add(controller)
    controller.set_command(0.0, 0.0, 1.0)
    sim.run(0.1)
    assert controller.command == pytest.approx([0.0, 0.0, 1.0])
