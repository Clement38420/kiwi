use kd_tree::KdSlice;
use shared::messages::{LidarPoint, RobotPose};
use std::f32::consts::PI;
use std::sync::Arc;
use tokio::sync::watch;
use tokio::sync::watch::Sender;

const ICP_MAX_ITERATIONS: usize = 10;
const ICP_T_THRESHOLD: f32 = 0.0005 * 0.0005;
const ICP_R_THRESHOLD: f32 = 0.0002 * 0.0002;
const MOVEMENT_T_THRESHOLD: f32 = 0.01 * 0.01;
const MOVEMENT_R_THRESHOLD: f32 = 0.017;

pub trait LidarPointExt {
    fn to_euclidean(&self) -> [f32; 2];
}

impl LidarPointExt for LidarPoint {
    fn to_euclidean(&self) -> [f32; 2] {
        let (s, c) = self.angle.sin_cos();
        [c * self.distance, -s * self.distance]
    }
}

fn preprocess_scan(scan: &[LidarPoint]) -> Vec<[f32; 2]> {
    scan.iter()
        .filter(|p| p.distance > 0.05 && p.distance < 16.0)
        .map(LidarPointExt::to_euclidean)
        .collect()
}

pub async fn odometry_task(
    mut lidar_rx: watch::Receiver<Arc<Vec<LidarPoint>>>,
    robot_pose_tx: Sender<RobotPose>,
    odom_rx: watch::Receiver<RobotPose>,
) {
    let _ = lidar_rx.changed().await;
    let mut prev_pc = preprocess_scan(&lidar_rx.borrow_and_update());
    let mut prev_odom = *odom_rx.borrow();

    let mut matches: Vec<Option<([f32; 2], f32)>> = Vec::new();

    loop {
        let _ = lidar_rx.changed().await;
        let raw_pc = preprocess_scan(&lidar_rx.borrow_and_update());
        // Check if there are enough points to compute R and t
        if raw_pc.len() < 10 || prev_pc.len() < 10 {
            continue;
        }

        let current_odom = *odom_rx.borrow();
        let dtheta_prior = (current_odom.theta - prev_odom.theta + PI).rem_euclid(2.0 * PI) - PI;
        let (sin_prior, cos_prior) = dtheta_prior.sin_cos();
        let (sin_a, cos_a) = prev_odom.theta.sin_cos();
        let dx_prior =
            cos_a * (current_odom.x - prev_odom.x) + sin_a * (current_odom.y - prev_odom.y);
        let dy_prior =
            -sin_a * (current_odom.x - prev_odom.x) + cos_a * (current_odom.y - prev_odom.y);

        // Pre-transform points using IMU gyro delta as motion prior
        let mut pc = raw_pc.clone();
        for p in &mut pc {
            let (x, y) = (p[0], p[1]);
            p[0] = cos_prior * x - sin_prior * y + dx_prior;
            p[1] = sin_prior * x + cos_prior * y + dy_prior;
        }

        let prev_tree = KdSlice::sort_by_ordered_float(&mut prev_pc);

        let mut scan_dx = dx_prior;
        let mut scan_dy = dy_prior;
        let mut scan_dtheta = dtheta_prior;

        // ICP Loop
        for _ in 0..ICP_MAX_ITERATIONS {
            matches.clear();
            matches.resize(prev_tree.len(), None);

            for &p in &pc {
                if let Some(n) = prev_tree.nearest(&p)
                    && n.squared_distance < 0.2 * 0.2
                {
                    let idx = (std::ptr::from_ref::<[f32; 2]>(n.item) as usize
                        - prev_tree.as_ptr() as usize)
                        / size_of::<[f32; 2]>();

                    match matches[idx] {
                        Some((_, best_dist)) if n.squared_distance >= best_dist => {}
                        _ => matches[idx] = Some((p, n.squared_distance)),
                    }
                }
            }

            let mut sum_px = 0.0f32;
            let mut sum_py = 0.0f32;
            let mut sum_qx = 0.0f32;
            let mut sum_qy = 0.0f32;
            let mut count = 0usize;

            for (idx, m) in matches.iter().enumerate() {
                if let Some((p, _)) = m {
                    let q = prev_tree[idx];
                    sum_px += p[0];
                    sum_py += p[1];
                    sum_qx += q[0];
                    sum_qy += q[1];
                    count += 1;
                }
            }

            // Not enough points to compute R and t
            if count < 10 {
                break;
            }

            let inv_n = 1.0 / (count as f32);
            let (mean_px, mean_py) = (sum_px * inv_n, sum_py * inv_n);
            let (mean_qx, mean_qy) = (sum_qx * inv_n, sum_qy * inv_n);

            let mut a = 0.0f32;
            let mut b = 0.0f32;

            for (idx, m) in matches.iter().enumerate() {
                if let Some((p, _)) = m {
                    let q = prev_tree[idx];
                    let px = p[0] - mean_px;
                    let py = p[1] - mean_py;
                    let qx = q[0] - mean_qx;
                    let qy = q[1] - mean_qy;

                    a += px * qx + py * qy; // Scalar product
                    b += px * qy - py * qx; // Cross product
                }
            }

            let dtheta = b.atan2(a);
            let (sin_dtheta, cos_dtheta) = dtheta.sin_cos();
            let dx = mean_qx - (cos_dtheta * mean_px - sin_dtheta * mean_py);
            let dy = mean_qy - (sin_dtheta * mean_px + cos_dtheta * mean_py);

            for p in &mut pc {
                let x = p[0];
                let y = p[1];
                p[0] = cos_dtheta * x - sin_dtheta * y + dx;
                p[1] = sin_dtheta * x + cos_dtheta * y + dy;
            }

            let (prev_dx, prev_dy) = (scan_dx, scan_dy);
            scan_dx = cos_dtheta * prev_dx - sin_dtheta * prev_dy + dx;
            scan_dy = sin_dtheta * prev_dx + cos_dtheta * prev_dy + dy;
            scan_dtheta += dtheta;

            if (dx * dx + dy * dy) < ICP_T_THRESHOLD && dtheta.abs() < ICP_R_THRESHOLD {
                break;
            }
        }

        if (scan_dx * scan_dx + scan_dy * scan_dy) > MOVEMENT_T_THRESHOLD
            || scan_dtheta.abs() > MOVEMENT_R_THRESHOLD
        {
            robot_pose_tx.send_modify(|pose| {
                let (sin, cos) = pose.theta.sin_cos();
                pose.x += cos * scan_dx - sin * scan_dy;
                pose.y += sin * scan_dx + cos * scan_dy;
                pose.theta = current_odom.theta;
            });

            prev_pc = raw_pc;
            prev_odom = current_odom;
        }
    }
}
