from data import Data


class Simulator:
    def __init__(self, dt=0.05):
        self.dt = dt
        self.time = 0.0
        self.data = Data()
        self.modules = []

    def add(self, module):
        self.modules.append(module)
        self.modules.sort(key=lambda module: module.priority, reverse=True)

    def step(self):
        self.time += self.dt
        for module in self.modules:
            module.step(self)

    def run(self, duration):
        for module in self.modules:
            module.setup(self)
        while self.time < duration - 1e-9:
            self.step()
