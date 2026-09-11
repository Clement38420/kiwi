from abc import ABC, abstractmethod

PRIORITY_CONTROLLER = 200
PRIORITY_PLANT = 100
PRIORITY_SENSOR = 50
PRIORITY_ESTIMATOR = 10
PRIORITY_LOGGER = 0


class Module(ABC):
    priority: int = 0

    def setup(self, sim):
        pass

    @abstractmethod
    def step(self, sim):
        ...
