use kd_tree::KdTree2;
use shared::geometry::{Point2D, PointPolar, Pose2D};
use shared::messages::LidarPoint;
use std::f32::consts::PI;
use std::sync::Arc;
use tokio::sync::watch::Sender;
use tokio::sync::{broadcast, watch};

const ICP_MAX_ITERATIONS: usize = 10;
const ICP_T_THRESHOLD: f32 = 0.0005 * 0.0005;
const ICP_R_THRESHOLD: f32 = 0.0002 * 0.0002;
const KEYFRAME_T_THRESHOLD: f32 = 0.15 * 0.15; // 15 cm
const KEYFRAME_R_THRESHOLD: f32 = 0.35; // ~20 degrees
const KEYFRAME_MATCHES_THRESHOLD: f32 = 0.6;

fn preprocess_scan(scan: &[LidarPoint]) -> Vec<[f32; 2]> {
    scan.iter()
        .filter(|p| p.distance > 0.05 && p.distance < 16.0)
        .map(|p| Point2D::from(PointPolar::from(p)).into())
        .collect()
}

struct Keyframe {
    tree: KdTree2<[f32; 2]>,
    pose: Pose2D,
    odom: Pose2D,
}

struct IcpResult {
    dx: f32,
    dy: f32,
    dtheta: f32,
    matched_points: usize,
}

impl Keyframe {
    fn new(pc: Vec<[f32; 2]>, pose: Pose2D, odom: Pose2D) -> Self {
        let tree = KdTree2::build_by_ordered_float(pc);
        Self { tree, pose, odom }
    }

    /// Aligns `raw_pc` against this keyframe using motion prior from `current_odom` and 2D ICP.
    fn align(
        &self,
        raw_pc: &[[f32; 2]],
        current_odom: &Pose2D,
        matches: &mut Vec<Option<([f32; 2], f32)>>,
    ) -> Option<IcpResult> {
        if raw_pc.len() < 10 || self.tree.len() < 10 {
            return None;
        }

        // 1. Motion prior from MCU relative to the keyframe
        let (sin_kf_odom, cos_kf_odom) = self.odom.theta.sin_cos();
        let dx_prior = cos_kf_odom * (current_odom.x - self.odom.x)
            + sin_kf_odom * (current_odom.y - self.odom.y);
        let dy_prior = -sin_kf_odom * (current_odom.x - self.odom.x)
            + cos_kf_odom * (current_odom.y - self.odom.y);

        let dtheta_prior = (current_odom.theta - self.odom.theta + PI).rem_euclid(2.0 * PI) - PI;
        let (sin_prior, cos_prior) = dtheta_prior.sin_cos();

        let mut pc: Vec<[f32; 2]> = raw_pc
            .iter()
            .map(|&[x, y]| {
                [
                    cos_prior * x - sin_prior * y + dx_prior,
                    sin_prior * x + cos_prior * y + dy_prior,
                ]
            })
            .collect();

        let mut scan_dx = dx_prior;
        let mut scan_dy = dy_prior;
        let mut scan_dtheta = dtheta_prior;
        let mut last_match_count = 0usize;

        // 2. ICP Loop
        for icp_iter in 0..ICP_MAX_ITERATIONS {
            matches.clear();
            matches.resize(self.tree.len(), None);

            let adaptive_distance_threshold = (0.30 * 0.75f32.powi(icp_iter as i32)).max(0.04);
            let threshold_sq = adaptive_distance_threshold * adaptive_distance_threshold;

            for &p in &pc {
                if let Some(n) = self.tree.nearest(&p)
                    && n.squared_distance < threshold_sq
                {
                    let idx = (std::ptr::from_ref::<[f32; 2]>(n.item) as usize
                        - self.tree.as_ptr() as usize)
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
                    let q = self.tree[idx];
                    sum_px += p[0];
                    sum_py += p[1];
                    sum_qx += q[0];
                    sum_qy += q[1];
                    count += 1;
                }
            }

            last_match_count = count;

            if count < 10 {
                return None;
            }

            let inv_n = 1.0 / (count as f32);
            let (mean_px, mean_py) = (sum_px * inv_n, sum_py * inv_n);
            let (mean_qx, mean_qy) = (sum_qx * inv_n, sum_qy * inv_n);

            let mut a = 0.0f32;
            let mut b = 0.0f32;

            for (idx, m) in matches.iter().enumerate() {
                if let Some((p, _)) = m {
                    let q = self.tree[idx];
                    let px = p[0] - mean_px;
                    let py = p[1] - mean_py;
                    let qx = q[0] - mean_qx;
                    let qy = q[1] - mean_qy;

                    a += px * qx + py * qy;
                    b += px * qy - py * qx;
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

        Some(IcpResult {
            dx: scan_dx,
            dy: scan_dy,
            dtheta: scan_dtheta,
            matched_points: last_match_count,
        })
    }

    /// Evaluates whether the robot has moved enough or lost enough overlap to spawn a new keyframe.
    fn should_replace(&self, icp: &IcpResult) -> bool {
        let traveled_dist_sq = icp.dx * icp.dx + icp.dy * icp.dy;
        traveled_dist_sq > KEYFRAME_T_THRESHOLD
            || icp.dtheta.abs() > KEYFRAME_R_THRESHOLD
            || (icp.matched_points as f32) < (self.tree.len() as f32 * KEYFRAME_MATCHES_THRESHOLD)
    }
}

pub async fn odometry_task(
    mut lidar_rx: watch::Receiver<Arc<Vec<LidarPoint>>>,
    robot_pose_tx: Sender<Pose2D>,
    mut odom_rx: watch::Receiver<Pose2D>,
    mut reset_rx: broadcast::Receiver<()>,
) {
    let _ = lidar_rx.changed().await;
    let initial_pc = preprocess_scan(&lidar_rx.borrow_and_update());
    let initial_pose = *robot_pose_tx.borrow();
    let initial_odom = *odom_rx.borrow();

    let mut keyframe = Keyframe::new(initial_pc, initial_pose, initial_odom);
    let mut matches: Vec<Option<([f32; 2], f32)>> = Vec::new();

    loop {
        let raw_pc = tokio::select! {
            result = reset_rx.recv() => {
                if result.is_ok() {
                    let zero_pose = Pose2D::default();
                    let current_pc = preprocess_scan(&lidar_rx.borrow_and_update());
                    let _ = odom_rx.borrow_and_update();
                    keyframe = Keyframe::new(current_pc, zero_pose, zero_pose);
                    let _ = robot_pose_tx.send(zero_pose);
                }
                continue;
            }
            _ = odom_rx.changed() => {
                let current_odom = *odom_rx.borrow_and_update();
                robot_pose_tx.send_modify(|pose| {
                    pose.theta = current_odom.theta;
                });
                continue;
            }
            _ = lidar_rx.changed() => {
                preprocess_scan(&lidar_rx.borrow_and_update())
            }
        };

        let current_odom = *odom_rx.borrow();
        let Some(icp) = keyframe.align(&raw_pc, &current_odom, &mut matches) else {
            robot_pose_tx.send_modify(|pose| {
                pose.theta = current_odom.theta;
            });
            if raw_pc.len() >= 10 {
                keyframe = Keyframe::new(raw_pc, *robot_pose_tx.borrow(), current_odom);
            }
            continue;
        };

        // Project displacement relative to the keyframe into world frame
        let (sin_kf, cos_kf) = keyframe.pose.theta.sin_cos();
        let world_x = keyframe.pose.x + cos_kf * icp.dx - sin_kf * icp.dy;
        let world_y = keyframe.pose.y + sin_kf * icp.dx + cos_kf * icp.dy;

        robot_pose_tx.send_modify(|pose| {
            pose.x = world_x;
            pose.y = world_y;
            pose.theta = current_odom.theta;
        });

        if keyframe.should_replace(&icp) {
            let new_keyframe_pose = Pose2D {
                x: world_x,
                y: world_y,
                theta: current_odom.theta,
            };
            keyframe = Keyframe::new(raw_pc, new_keyframe_pose, current_odom);
        }
    }
}
