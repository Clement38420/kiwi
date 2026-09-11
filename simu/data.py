from dataclasses import dataclass

import numpy as np


@dataclass
class Data:
    command: np.ndarray | None = None
    true_pose: np.ndarray | None = None
    odom_twist: np.ndarray | None = None
    odom_pose: np.ndarray | None = None
