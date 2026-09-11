import matplotlib.pyplot as plt
import numpy as np
from matplotlib.animation import FuncAnimation


class Renderer:
    def __init__(
        self,
        series,
        x_lim=None,
        y_lim=None,
        arrow_len=0.1,
        speed=2.0,
    ):
        self.arrow_len = arrow_len
        self.times = np.asarray([t for t, _ in series[0][1]])
        self.poses = [
            np.asarray([pose for _, pose in history]) for _, history, _ in series
        ]
        self.interval = (self.times[1] - self.times[0]) * 1000 / speed

        if x_lim is None or y_lim is None:
            points = np.vstack(self.poses)[:, :2]
            low = points.min(axis=0) - 0.5
            high = points.max(axis=0) + 0.5
            x_lim = x_lim or (low[0], high[0])
            y_lim = y_lim or (low[1], high[1])

        self.fig, self.ax = plt.subplots()
        self.ax.set_aspect("equal")
        self.ax.set_xlim(*x_lim)
        self.ax.set_ylim(*y_lim)
        self.ax.grid(True)

        self.lines = []
        self.heads = []
        self.arrows = []
        for label, _, style in series:
            (line,) = self.ax.plot([], [], style, lw=1, label=label)
            color = line.get_color()
            (head,) = self.ax.plot([], [], "o", color=color)
            (arrow,) = self.ax.plot([], [], "-", color=color, lw=2)
            self.lines.append(line)
            self.heads.append(head)
            self.arrows.append(arrow)
        self.ax.legend(loc="upper left")
        self.title = self.ax.set_title("")

    def render(self, i):
        for line, head, arrow, poses in zip(
            self.lines, self.heads, self.arrows, self.poses
        ):
            path = poses[: i + 1]
            line.set_data(path[:, 0], path[:, 1])
            x, y, theta = poses[i]
            head.set_data([x], [y])
            arrow.set_data(
                [x, x + self.arrow_len * np.cos(theta)],
                [y, y + self.arrow_len * np.sin(theta)],
            )
        self.title.set_text(f"t={self.times[i]:.2f}s")
        return (*self.lines, *self.heads, *self.arrows, self.title)

    def run(self):
        n = min(len(poses) for poses in self.poses)
        self._ani = FuncAnimation(
            self.fig,
            self.render,
            frames=n,
            interval=self.interval,
            blit=False,
            cache_frame_data=False,
        )
        plt.show()
