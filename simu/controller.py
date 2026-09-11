import numpy as np

from module import PRIORITY_CONTROLLER, Module


class ScheduleController(Module):
    priority = PRIORITY_CONTROLLER

    def __init__(self, commands=()):
        self.commands = []
        t = 0.0
        for duration, cmd in commands:
            self.commands.append((t, np.asarray(cmd, dtype=float)))
            t += duration
        self.end_time = t
        self.command = np.zeros(3)
        self._next = 0

    def _apply(self, time):
        while (
            self._next < len(self.commands)
            and self.commands[self._next][0] <= time + 1e-9
        ):
            self.command = self.commands[self._next][1]
            self._next += 1

    def set_command(self, vx, vy, w):
        self.commands.clear()
        self._next = 0
        self.command = np.array([vx, vy, w], dtype=float)

    def setup(self, sim):
        self._apply(sim.time)
        sim.data.command = self.command.copy()

    def step(self, sim):
        self._apply(sim.time)
        sim.data.command = self.command.copy()
