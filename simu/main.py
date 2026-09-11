import math

from controller import ScheduleController
from loggers import PoseLogger
from plant import Plant
from renderer import Renderer
from simulator import Simulator
from wheel_odometry import WheelOdometry

DT = 0.05

MOVES = [
    (2.0, (0.5, 0.0, 0.0)),  # ligne droite
    (1.0, (0.0, 0.0, math.pi / 2)),  # rotation pure
    (2.0, (0.4, 0.0, 0.4)),  # courbe
    (1.5, (0.0, 0.4, 0.0)),  # translation laterale (holonome)
]

sim = Simulator(dt=DT)
controller = ScheduleController(MOVES)
sim.add(controller)
sim.add(Plant(0.0, 0.0, 0.0))
sim.add(WheelOdometry())

log_true = PoseLogger("true_pose")
log_odom = PoseLogger("odom_pose")
sim.add(log_true)
sim.add(log_odom)
sim.run(controller.end_time)

series = [
    ("vrai", log_true.history, "b-"),
    ("odom", log_odom.history, "tab:orange"),
]
Renderer(series, speed=1.0).run()