import matplotlib.pyplot as plt
import numpy as np
from matplotlib.animation import FuncAnimation

from robot import Robot

DT = 0.05

robot = Robot()
robot.command(0.5, 0.0, 0.0)

fig, ax = plt.subplots()
ax.set_aspect("equal")
ax.set_xlim(-0.5, 6)
ax.set_ylim(-2, 2)
ax.grid(True)
(line,) = ax.plot([], [], "b-", lw=1)
body, = ax.plot([], [], "ro")


def update(_):
    robot.step(DT)
    path = np.array(robot.history)
    line.set_data(path[:, 0], path[:, 1])
    body.set_data([path[-1, 0]], [path[-1, 1]])
    return line, body


_ani = FuncAnimation(
    fig, update, interval=DT * 1000, blit=True, cache_frame_data=False
)
plt.show()
