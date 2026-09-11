import matplotlib.pyplot as plt
import numpy as np
from matplotlib.animation import FuncAnimation


class Renderer:
    def __init__(
        self,
        history,
        x_lim=(-2.0, 2.0),
        y_lim=(-2.0, 2.0),
        arrow_len=0.2,
        speed=1.0,
    ):
        self.times = np.array([t for t, _ in history])
        self.poses = np.array([pose for _, pose in history])
        self.arrow_len = arrow_len
        self.interval = (self.times[1] - self.times[0]) * 1000 / speed

        self.fig, self.ax = plt.subplots()
        self.ax.set_aspect("equal")
        self.ax.set_xlim(*x_lim)
        self.ax.set_ylim(*y_lim)
        self.ax.grid(True)

        (self.line,) = self.ax.plot([], [], "b-", lw=1)
        (self.body,) = self.ax.plot([], [], "ro")
        (self.arrow,) = self.ax.plot([], [], "r-", lw=2)
        self.title = self.ax.set_title("")

    def render(self, i):
        path = self.poses[: i + 1]
        x, y, theta = self.poses[i]
        self.line.set_data(path[:, 0], path[:, 1])
        self.body.set_data([x], [y])
        self.arrow.set_data(
            [x, x + self.arrow_len * np.cos(theta)],
            [y, y + self.arrow_len * np.sin(theta)],
        )
        self.title.set_text(f"t={self.times[i]:.2f}s")
        return self.line, self.body, self.arrow, self.title

    def run(self):
        self._ani = FuncAnimation(
            self.fig,
            self.render,
            frames=len(self.poses),
            interval=self.interval,
            blit=True,
            cache_frame_data=False,
        )
        plt.show()