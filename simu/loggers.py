from module import PRIORITY_LOGGER, Module


class PoseLogger(Module):
    priority = PRIORITY_LOGGER

    def __init__(self, field):
        self.field = field
        self.history = []

    def _record(self, sim):
        value = getattr(sim.data, self.field)
        if value is not None:
            self.history.append((sim.time, value.copy()))

    def setup(self, sim):
        self._record(sim)

    def step(self, sim):
        self._record(sim)
