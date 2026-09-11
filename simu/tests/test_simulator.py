import pytest

from module import Module
from simulator import Simulator


class Recorder(Module):
    def __init__(self, priority, log):
        self.priority = priority
        self.log = log

    def setup(self, sim):
        self.log.append(f"setup-{self.priority}")

    def step(self, sim):
        self.log.append(f"step-{self.priority}")


def test_time_increments_by_dt():
    sim = Simulator(dt=0.1)
    sim.run(0.5)
    assert sim.time == pytest.approx(0.5)


def test_modules_run_in_priority_order():
    log = []
    sim = Simulator(dt=0.1)
    sim.add(Recorder(0, log))
    sim.add(Recorder(100, log))
    sim.add(Recorder(50, log))
    sim.run(0.1)
    assert log == [
        "setup-100",
        "setup-50",
        "setup-0",
        "step-100",
        "step-50",
        "step-0",
    ]
