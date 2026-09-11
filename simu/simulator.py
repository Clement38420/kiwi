import numpy as np


class Simulator:
    def __init__(self, robot, dt=0.05, commands=()):
        self.robot = robot
        self.dt = dt
        self.time = 0.0
        self.command = (0.0, 0.0, 0.0)
        self.commands = []
        t = 0.0
        for duration, cmd in commands:
            self.commands.append((t, cmd))
            t += duration
        self.end_time = t
        self.history = [(self.time, np.asarray(robot.pose, dtype=float).copy())]

    def set_command(self, vx, vy, w):
        self.command = (vx, vy, w)
        self.commands.clear()

    def _advance_schedule(self):
        while self.commands and self.commands[0][0] <= self.time:
            _, self.command = self.commands.pop(0)

    def step(self):
        self._advance_schedule()
        self.robot.command(*self.command)
        self.robot.step(self.dt)
        self.time += self.dt
        self.history.append((self.time, self.robot.pose.copy()))

    def run(self, duration=None):
        end = self.end_time if duration is None else duration
        while self.time < end - 1e-9:
            self.step()
        return self.history