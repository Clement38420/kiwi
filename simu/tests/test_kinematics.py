import math

import numpy as np
import pytest
from kinematics import WHEEL_ANGLES, fk_matrix, ik_matrix, integrate


def test_fk_inverts_ik():
    ik = ik_matrix(0.05, 0.15)
    fk = fk_matrix(0.05, 0.15)
    rng = np.random.default_rng(0)
    for _ in range(1000):
        twist = rng.uniform(-1.0, 1.0, 3)
        assert np.allclose(fk @ (ik @ twist), twist, atol=1e-9)


def test_wheel_angles_are_120_degrees_apart():
    assert WHEEL_ANGLES[0] == pytest.approx(math.pi)
    diffs = np.diff(np.append(WHEEL_ANGLES, WHEEL_ANGLES[0] + 2 * math.pi))
    assert np.allclose(diffs, 2 * math.pi / 3)


def test_pure_rotation_drives_all_wheels_equally():
    wheels = ik_matrix(0.05, 0.15) @ np.array([0.0, 0.0, 1.0])
    assert np.allclose(wheels, wheels[0])


def test_translation_x_ignores_wheel_1():
    wheels = ik_matrix(0.05, 0.15) @ np.array([1.0, 0.0, 0.0])
    assert wheels[0] == pytest.approx(0.0)
    assert wheels[1] == pytest.approx(-wheels[2])


def test_integrate_straight_line():
    pose = np.array([0.0, 0.0, 0.0])
    for _ in range(20):
        integrate(pose, np.array([0.5, 0.0, 0.0]), 0.05)
    assert pose == pytest.approx([0.5, 0.0, 0.0])
