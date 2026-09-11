import math

from renderer import Renderer
from robot import Robot
from simulator import Simulator

robot = Robot(0.0, 0.0, 0.0)

commands = [
    (1.0, (0.0, 0.0, 0.0)),
    (2.0, (0.5, 0.0, 0.0)),
    (2.0, (0.0, 0.0, math.pi / 2)),
    (1.0, (0.0, 0.0, 0.0)),
]

sim = Simulator(robot, dt=0.05, commands=commands)
sim.run()
print(f"simulé {sim.time:.2f}s, {len(sim.history)} points")

Renderer(sim.history, speed=1.0).run()