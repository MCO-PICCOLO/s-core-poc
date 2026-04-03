/********************************************************************************
 * Copyright (c) 2025 Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use crate::activities::input::input;
use crate::activities::output::output;
#[cfg(feature = "com_mw")]
use com_api::{LolaRuntimeImpl, Runtime, SampleContainer, Subscriber};
use core::hash::{BuildHasher as _, Hasher as _};
use core::mem::MaybeUninit;
use core::ops::DerefMut;
use core::ops::{Deref, Range};
use core::time::Duration;
use feo::activity::Activity;
use feo::error::ActivityError;
use feo::ids::ActivityId;
use feo_com::interface::{ActivityInput, ActivityOutput};
use feo_tracing::instrument;
#[cfg(feature = "com_mw")]
use mini_adas_gen::{
    BrakeControllerConsumer, BrakeControllerInterface, BrakeControllerOfferedProducer, CameraConsumer, CameraInterface,
    CameraOfferedProducer, NeuralNetConsumer, NeuralNetInterface, NeuralNetOfferedProducer, RadarConsumer,
    RadarInterface, RadarOfferedProducer, SteeringControllerConsumer, SteeringControllerInterface,
    SteeringControllerOfferedProducer,
};
use mini_adas_gen::{BrakeInstruction, CameraImage, RadarScan, Scene, Steering};
use score_log::{debug, info};
use std::collections::HashMap;
use std::hash::RandomState;
use std::sync::{Mutex, OnceLock};
use std::thread;

const SLEEP_RANGE: Range<i64> = 10..45;

struct DelayInjectionConfig {
    every_cycles: u64,
    delay_ms: u64,
    target_activity_ids: Vec<u64>,
}

#[cfg(feature = "com_mw")]
type MwComSubscriber<T> = <LolaRuntimeImpl as Runtime>::Subscriber<T>;

/// Camera activity
///
/// This activity emulates a camera generating a [CameraImage].
#[derive(Debug)]
pub struct Camera {
    /// ID of the activity
    activity_id: ActivityId,
    /// Image output
    output_image: Box<dyn ActivityOutput<CameraImage>>,
    // Local state for pseudo-random output generation
    num_people: usize,
    num_cars: usize,
    distance_obstacle: f64,
}

impl Camera {
    pub fn build(activity_id: ActivityId, image_topic: &str) -> Box<dyn Activity> {
        let output_image = output!(CameraInterface, image_topic, |i: CameraOfferedProducer<_>| i.image);
        Box::new(Self {
            activity_id,
            output_image,
            num_people: 4,
            num_cars: 10,
            distance_obstacle: 40.0,
        })
    }

    fn get_image(&mut self) -> CameraImage {
        const PEOPLE_CHANGE_PROP: f64 = 0.8;
        const CAR_CHANGE_PROP: f64 = 0.8;
        const DISTANCE_CHANGE_PROP: f64 = 1.0;

        self.num_people = random_walk_integer(self.num_people, PEOPLE_CHANGE_PROP, 1);
        self.num_cars = random_walk_integer(self.num_people, CAR_CHANGE_PROP, 2);
        let sample = random_walk_float(self.distance_obstacle, DISTANCE_CHANGE_PROP, 5.0);
        self.distance_obstacle = sample.clamp(20.0, 50.0);

        CameraImage {
            num_people: self.num_people,
            num_cars: self.num_cars,
            distance_obstacle: self.distance_obstacle,
        }
    }
}

impl Activity for Camera {
    fn id(&self) -> ActivityId {
        self.activity_id
    }

    #[instrument(name = "Camera startup")]
    fn startup(&mut self) -> Result<(), ActivityError> {
        Ok(())
    }

    #[instrument(name = "Camera")]
    fn step(&mut self) -> Result<(), ActivityError> {
        debug!("Stepping Camera");
        sleep_random(self.activity_id);

        let image = self.get_image();
        debug!("Sending image: {:?}", image);
        self.output_image
            .write_uninit()
            .unwrap()
            .write_payload(image)
            .send()
            .unwrap();
        Ok(())
    }

    #[instrument(name = "Camera shutdown")]
    fn shutdown(&mut self) -> Result<(), ActivityError> {
        debug!("Shutting down Camera activity {}", self.activity_id);
        Ok(())
    }
}

/// Radar activity
///
/// This component emulates are radar generating a [RadarScan].
#[derive(Debug)]
pub struct Radar {
    /// ID of the activity
    activity_id: ActivityId,
    /// Radar scan output
    output_scan: Box<dyn ActivityOutput<RadarScan>>,

    // Local state for pseudo-random output generation
    distance_obstacle: f64,
}

impl Radar {
    pub fn build(activity_id: ActivityId, radar_topic: &str) -> Box<dyn Activity> {
        let output_scan = output!(RadarInterface, radar_topic, |r: RadarOfferedProducer<_>| r.scan);
        Box::new(Self {
            activity_id,
            output_scan,
            distance_obstacle: 40.0,
        })
    }

    fn get_scan(&mut self) -> RadarScan {
        const DISTANCE_CHANGE_PROP: f64 = 1.0;

        let sample = random_walk_float(self.distance_obstacle, DISTANCE_CHANGE_PROP, 6.0);
        self.distance_obstacle = sample.clamp(16.0, 60.0);

        let error_margin = gen_random_in_range(-10..10) as f64 / 10.0;

        RadarScan {
            distance_obstacle: self.distance_obstacle,
            error_margin,
        }
    }
}

impl Activity for Radar {
    fn id(&self) -> ActivityId {
        self.activity_id
    }

    #[instrument(name = "Radar startup")]
    fn startup(&mut self) -> Result<(), ActivityError> {
        Ok(())
    }

    #[instrument(name = "Radar")]
    fn step(&mut self) -> Result<(), ActivityError> {
        debug!("Stepping Radar");
        sleep_random(self.activity_id);

        let scan = self.get_scan();
        debug!("Sending scan: {:?}", scan);
        self.output_scan
            .write_uninit()
            .unwrap()
            .write_payload(scan)
            .send()
            .unwrap();
        Ok(())
    }

    #[instrument(name = "Radar shutdown")]
    fn shutdown(&mut self) -> Result<(), ActivityError> {
        debug!("Shutting down Radar activity {}", self.activity_id);
        Ok(())
    }
}

/// Neural network activity
///
/// This component emulates a neural network
/// pseudo-inferring a [Scene] output
/// from the provided [Camera] and [Radar] inputs.
#[derive(Debug)]
pub struct NeuralNet {
    /// ID of the activity
    activity_id: ActivityId,
    /// Image input
    input_image: Box<dyn ActivityInput<CameraImage>>,
    /// Radar scan input
    input_scan: Box<dyn ActivityInput<RadarScan>>,
    /// Scene output
    output_scene: Box<dyn ActivityOutput<Scene>>,
}

impl NeuralNet {
    pub fn build(activity_id: ActivityId, image_topic: &str, scan_topic: &str, scene_topic: &str) -> Box<dyn Activity> {
        let output_scene = output!(NeuralNetInterface, scene_topic, |s: NeuralNetOfferedProducer<_>| s
            .scene);
        Box::new(Self {
            activity_id,
            input_image: input!(
                CameraInterface,
                image_topic,
                |i: CameraConsumer<_>| -> MwComSubscriber<CameraImage> { i.image }
            ),
            input_scan: input!(
                RadarInterface,
                scan_topic,
                |r: RadarConsumer<_>| -> MwComSubscriber<RadarScan> { r.scan }
            ),
            output_scene,
        })
    }

    fn infer(image: &CameraImage, radar: &RadarScan, scene: &mut MaybeUninit<Scene>) {
        let CameraImage {
            num_people,
            num_cars,
            distance_obstacle,
        } = *image;

        let distance_obstacle = distance_obstacle.min(radar.distance_obstacle);
        let distance_left_lane = gen_random_in_range(5..10) as f64 / 10.0;
        let distance_right_lane = gen_random_in_range(5..10) as f64 / 10.0;

        // Get raw pointer to payload within `MaybeUninit`.
        let scene_ptr = scene.as_mut_ptr();

        // Safety: `scene_ptr` was create from a `MaybeUninit` of the right type and size.
        // The underlying type `Scene` has `repr(C)` and can be populated field by field.
        unsafe {
            (*scene_ptr).num_people = num_people;
            (*scene_ptr).num_cars = num_cars;
            (*scene_ptr).distance_obstacle = distance_obstacle;
            (*scene_ptr).distance_left_lane = distance_left_lane;
            (*scene_ptr).distance_right_lane = distance_right_lane;
        }
    }
}

impl Activity for NeuralNet {
    fn id(&self) -> ActivityId {
        self.activity_id
    }

    #[instrument(name = "NeuralNet startup")]
    fn startup(&mut self) -> Result<(), ActivityError> {
        Ok(())
    }

    #[instrument(name = "NeuralNet")]
    fn step(&mut self) -> Result<(), ActivityError> {
        debug!("Stepping NeuralNet");
        sleep_random(self.activity_id);

        let camera = self.input_image.read().unwrap();
        let radar = self.input_scan.read().unwrap();

        debug!("Inferring scene with neural network");

        let mut scene = self.output_scene.write_uninit().unwrap();
        Self::infer(&camera, &radar, scene.deref_mut());
        // Safety: `Scene` has `repr(C)` and was fully initialized by `Self::infer` above.
        let scene = unsafe { scene.assume_init() };
        debug!("Sending Scene {:?}", scene.deref());
        scene.send().unwrap();
        Ok(())
    }

    #[instrument(name = "NeuralNet shutdown")]
    fn shutdown(&mut self) -> Result<(), ActivityError> {
        debug!("Shutting down NeuralNet activity {}", self.activity_id);
        Ok(())
    }
}

/// Emergency braking activity
///
/// This component emulates an emergency braking function
/// which sends instructions to activate the brakes
/// if the distance to the closest obstacle becomes too small.
/// The level of brake engagement depends on the distance.
#[derive(Debug)]
pub struct EmergencyBraking {
    /// ID of the activity
    activity_id: ActivityId,
    /// Scene input
    input_scene: Box<dyn ActivityInput<Scene>>,
    /// Brake instruction output
    output_brake_instruction: Box<dyn ActivityOutput<BrakeInstruction>>,
}

impl EmergencyBraking {
    pub fn build(activity_id: ActivityId, scene_topic: &str, brake_instruction_topic: &str) -> Box<dyn Activity> {
        let output_brake_instruction = output!(
            BrakeControllerInterface,
            brake_instruction_topic,
            |b: BrakeControllerOfferedProducer<_>| b.brake_instruction
        );
        Box::new(Self {
            activity_id,
            input_scene: input!(
                NeuralNetInterface,
                scene_topic,
                |n: NeuralNetConsumer<_>| -> MwComSubscriber<Scene> { n.scene }
            ),
            output_brake_instruction,
        })
    }
}

impl Activity for EmergencyBraking {
    fn id(&self) -> ActivityId {
        self.activity_id
    }

    #[instrument(name = "EmergencyBraking startup")]
    fn startup(&mut self) -> Result<(), ActivityError> {
        Ok(())
    }

    #[instrument(name = "EmergencyBraking")]
    fn step(&mut self) -> Result<(), ActivityError> {
        debug!("Stepping EmergencyBraking");
        sleep_random(self.activity_id);

        let scene = self.input_scene.read().unwrap();
        let brake_instruction = self.output_brake_instruction.write_uninit().unwrap();

        const ENGAGE_DISTANCE: f64 = 30.0;
        const MAX_BRAKE_DISTANCE: f64 = 15.0;

        if scene.distance_obstacle < ENGAGE_DISTANCE {
            // Map distances ENGAGE_DISTANCE..MAX_BRAKE_DISTANCE to intensities 0.0..1.0
            let level = f64::min(
                1.0,
                (ENGAGE_DISTANCE - scene.distance_obstacle) / (ENGAGE_DISTANCE - MAX_BRAKE_DISTANCE),
            );

            let brake_instruction = brake_instruction.write_payload(BrakeInstruction { active: true, level });
            brake_instruction.send().unwrap();
        } else {
            let brake_instruction = brake_instruction.write_payload(BrakeInstruction {
                active: false,
                level: 0.0,
            });
            brake_instruction.send().unwrap();
        }
        Ok(())
    }

    #[instrument(name = "EmergencyBraking shutdown")]
    fn shutdown(&mut self) -> Result<(), ActivityError> {
        debug!("Shutting down EmergencyBraking activity {}", self.activity_id);
        Ok(())
    }
}

/// Brake controller activity
///
/// This component emulates a brake controller
/// which triggers the brakes based on an instruction
/// and therefore might run in a separate process
/// with only other ASIL-D activities.
#[derive(Debug)]
pub struct BrakeController {
    /// ID of the activity
    activity_id: ActivityId,
    /// Brake instruction input
    input_brake_instruction: Box<dyn ActivityInput<BrakeInstruction>>,
}

impl BrakeController {
    pub fn build(activity_id: ActivityId, brake_instruction_topic: &str) -> Box<dyn Activity> {
        Box::new(Self {
            activity_id,
            input_brake_instruction: input!(
                BrakeControllerInterface,
                brake_instruction_topic,
                |b: BrakeControllerConsumer<_>| -> MwComSubscriber<BrakeInstruction> { b.brake_instruction }
            ),
        })
    }
}

impl Activity for BrakeController {
    fn id(&self) -> ActivityId {
        self.activity_id
    }

    #[instrument(name = "BrakeController startup")]
    fn startup(&mut self) -> Result<(), ActivityError> {
        Ok(())
    }

    #[instrument(name = "BrakeController")]
    fn step(&mut self) -> Result<(), ActivityError> {
        debug!("Stepping BrakeController");
        sleep_random(self.activity_id);

        let brake_instruction = self.input_brake_instruction.read().unwrap();
        if brake_instruction.active {
            debug!(
                "BrakeController activating brakes with level {:.3}",
                brake_instruction.level
            )
        }
        Ok(())
    }

    #[instrument(name = "BrakeController shutdown")]
    fn shutdown(&mut self) -> Result<(), ActivityError> {
        debug!("Shutting down BrakeController activity {}", self.activity_id);
        Ok(())
    }
}

/// Environment renderer activity
///
/// This component emulates a renderer to display a scene
/// in the infotainment display.
/// In this example, it does not do anything with the scene input.
#[derive(Debug)]
pub struct EnvironmentRenderer {
    /// ID of the activity
    activity_id: ActivityId,
    /// Scene input
    input_scene: Box<dyn ActivityInput<Scene>>,
}

impl EnvironmentRenderer {
    pub fn build(activity_id: ActivityId, scene_topic: &str) -> Box<dyn Activity> {
        Box::new(Self {
            activity_id,
            input_scene: input!(
                NeuralNetInterface,
                scene_topic,
                |n: NeuralNetConsumer<_>| -> MwComSubscriber<Scene> { n.scene }
            ),
        })
    }
}

impl Activity for EnvironmentRenderer {
    fn id(&self) -> ActivityId {
        self.activity_id
    }

    #[instrument(name = "EnvironmentRenderer startup")]
    fn startup(&mut self) -> Result<(), ActivityError> {
        Ok(())
    }

    #[instrument(name = "EnvironmentRenderer")]
    fn step(&mut self) -> Result<(), ActivityError> {
        debug!("Stepping EnvironmentRenderer");
        sleep_random(self.activity_id);

        let _scene = self.input_scene.read();
        debug!("Rendering scene");
        Ok(())
    }

    #[instrument(name = "EnvironmentRenderer shutdown")]
    fn shutdown(&mut self) -> Result<(), ActivityError> {
        debug!("Shutting down EnvironmentRenderer activity {}", self.activity_id);
        Ok(())
    }
}

/// Steering controller activity
///
/// This component emulates a steering controller
/// which adjusts the steering angle to control the heading of the car.
/// Therefore, it might run in a separate process
/// with only other ASIL-D activities.
#[derive(Debug)]
pub struct SteeringController {
    /// ID of the activity
    activity_id: ActivityId,
    /// Steering input
    input_steering: Box<dyn ActivityInput<Steering>>,
}

impl SteeringController {
    pub fn build(activity_id: ActivityId, steering_topic: &str) -> Box<dyn Activity> {
        Box::new(Self {
            activity_id,
            input_steering: input!(
                SteeringControllerInterface,
                steering_topic,
                |s: SteeringControllerConsumer<_>| -> MwComSubscriber<Steering> { s.steering }
            ),
        })
    }
}

impl Activity for SteeringController {
    fn id(&self) -> ActivityId {
        self.activity_id
    }

    #[instrument(name = "SteeringController startup")]
    fn startup(&mut self) -> Result<(), ActivityError> {
        Ok(())
    }

    #[instrument(name = "SteeringController")]
    fn step(&mut self) -> Result<(), ActivityError> {
        debug!("Stepping SteeringController");
        sleep_random(self.activity_id);

        let steering = self.input_steering.read().unwrap();
        debug!("SteeringController adjusting angle to {:.3}", steering.angle);
        Ok(())
    }

    #[instrument(name = "SteeringController shutdown")]
    fn shutdown(&mut self) -> Result<(), ActivityError> {
        debug!("Shutting down SteeringController activity {}", self.activity_id);
        Ok(())
    }
}

/// LaneAssist stub activity
///
#[derive(Debug)]
pub struct LaneAssist {
    /// ID of the activity
    activity_id: ActivityId,
    /// Brake instruction output
    steering_controller: Box<dyn ActivityOutput<Steering>>,
}

impl LaneAssist {
    pub fn build(activity_id: ActivityId, steering_topic: &str) -> Box<dyn Activity> {
        let steering_controller = output!(
            SteeringControllerInterface,
            steering_topic,
            |s: SteeringControllerOfferedProducer<_>| s.steering
        );
        Box::new(Self {
            activity_id,
            steering_controller,
        })
    }
}

impl Activity for LaneAssist {
    fn id(&self) -> ActivityId {
        self.activity_id
    }

    #[instrument(name = "LaneAssist startup")]
    fn startup(&mut self) -> Result<(), ActivityError> {
        Ok(())
    }

    #[instrument(name = "LaneAssist")]
    fn step(&mut self) -> Result<(), ActivityError> {
        debug!("Stepping LaneAssist");
        sleep_random(self.activity_id);
        self.steering_controller
            .write_uninit()
            .unwrap()
            .write_payload(Steering { angle: 2.34 })
            .send()
            .unwrap();
        Ok(())
    }

    #[instrument(name = "LaneAssist shutdown")]
    fn shutdown(&mut self) -> Result<(), ActivityError> {
        Ok(())
    }
}

/// TrajectoryVisualizer stub activity
///
#[derive(Debug)]
pub struct TrajectoryVisualizer {
    /// ID of the activity
    activity_id: ActivityId,
}

impl TrajectoryVisualizer {
    pub fn build(activity_id: ActivityId) -> Box<dyn Activity> {
        Box::new(Self { activity_id })
    }
}

impl Activity for TrajectoryVisualizer {
    fn id(&self) -> ActivityId {
        self.activity_id
    }

    #[instrument(name = "TrajectoryVisualizer startup")]
    fn startup(&mut self) -> Result<(), ActivityError> {
        Ok(())
    }

    #[instrument(name = "TrajectoryVisualizer")]
    fn step(&mut self) -> Result<(), ActivityError> {
        debug!("Stepping TrajectoryVisualizer");
        sleep_random(self.activity_id);
        Ok(())
    }

    #[instrument(name = "TrajectoryVisualizer shutdown")]
    fn shutdown(&mut self) -> Result<(), ActivityError> {
        Ok(())
    }
}

/// DDS resources owned by [RearCamera]; declared as a separate struct so that
/// `DomainParticipant` (which does not implement `Debug`) can be hidden behind
/// a manual `Debug` impl without infecting the whole activity.
///
/// Field order matters for drop: writers first, participant last.
struct DdsResources {
    vehicle_state_writer:
        dust_dds::publication::data_writer::DataWriter<crate::dds_types::VehicleState>,
    rear_camera_writer:
        dust_dds::publication::data_writer::DataWriter<crate::dds_types::RearCameraScan>,
    /// Must be dropped *after* the writers above; keep this field last.
    _participant: dust_dds::domain::domain_participant::DomainParticipant,
}

impl std::fmt::Debug for DdsResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DdsResources").finish_non_exhaustive()
    }
}

/// Rear camera activity
///
/// Simulates a rear-facing camera and publishes vehicle state and obstacle data
/// over DDS (domain 100) inside the normal FEO step.  The DDS `write()` call is
/// UDP fire-and-forget so it never stalls the FEO sensor cycle.
///
/// **DDS topics:**
/// - `vehicle/state`      — [`crate::dds_types::VehicleState`], published every cycle
/// - `sensor/rear_camera` — [`crate::dds_types::RearCameraScan`], published when speed < 5 km/h
///
/// **Demo cycle (30 s, repeating):**
///
/// | Phase        | Time       | Speed      | Gear | Rear camera               |
/// |--------------|------------|------------|------|---------------------------|
/// | Decelerating | 0 – 15 s   | 50 → 0     | D    | off (speed ≥ 5 km/h)      |
/// | Slow / stop  | ~13.5 – 15 s | 5 → 0    | D    | obstacle approaching      |
/// | Just parked  | 15 – 20 s  | 0          | D    | obstacle approaching      |
/// | Parked (P)   | 20 – 23 s  | 0          | P    | obstacle approaching      |
/// | Parked (P)   | 23 – 30 s  | 0          | P    | clear — safe to exit      |
///
/// The obstacle scenario models something (cyclist, car) approaching from behind
/// while the vehicle parks.  Its distance decreases from 3.0 m → 0.8 m between
/// t ≈ 13.5 s and t = 23 s, then the path clears — exactly the moment a Safe Exit
/// app would give the green light to open a rear door.
#[derive(Debug)]
pub struct RearCamera {
    activity_id: ActivityId,
    /// Reset to `Instant::now()` in `startup` so the demo cycle starts cleanly.
    cycle_start: std::time::Instant,
    /// Initialised in `startup`; `None` before startup and after shutdown.
    dds: Option<DdsResources>,
}

impl RearCamera {
    pub fn build(activity_id: ActivityId) -> Box<dyn Activity> {
        Box::new(Self {
            activity_id,
            cycle_start: std::time::Instant::now(),
            dds: None,
        })
    }

    /// Compute the current [`VehicleState`] and, when speed < 5 km/h, a
    /// [`RearCameraScan`], purely from wall-clock elapsed time within the
    /// 30-second demo cycle.
    fn compute_state(
        &self,
    ) -> (
        crate::dds_types::VehicleState,
        Option<crate::dds_types::RearCameraScan>,
    ) {
        use crate::dds_types::{RearCameraScan, VehicleState};

        // ── Cycle constants ──────────────────────────────────────────────────
        const CYCLE_SECS: f32 = 30.0;
        /// Speed drops to 0 at this point.
        const DECEL_END: f32 = 15.0;
        /// Driver shifts to P this many seconds after speed = 0.
        const GEAR_PARK_AT: f32 = 20.0;
        /// Obstacle clears at this phase time.
        const OBSTACLE_CLEAR_AT: f32 = 23.0;
        /// Below this speed (km/h) rear-camera data is published.
        const SLOW_THRESHOLD: f32 = 5.0;
        /// Phase time when speed first crosses below SLOW_THRESHOLD.
        const SLOW_START_T: f32 = 13.5;

        let phase_t = self.cycle_start.elapsed().as_secs_f32() % CYCLE_SECS;

        // Speed: linear 50 → 0 over first 15 s, then held at 0.
        let speed = if phase_t < DECEL_END {
            50.0_f32 * (1.0 - phase_t / DECEL_END)
        } else {
            0.0_f32
        };

        // Gear: D while moving + first 5 s stationary; P after that.
        let gear: u8 = if phase_t < GEAR_PARK_AT { 1 } else { 0 };

        let rear_cam = if speed < SLOW_THRESHOLD {
            if phase_t < OBSTACLE_CLEAR_AT {
                // Something is approaching from behind.
                // Distance interpolates 3.0 m → 0.8 m between SLOW_START_T and OBSTACLE_CLEAR_AT.
                let interp = ((phase_t - SLOW_START_T) / (OBSTACLE_CLEAR_AT - SLOW_START_T))
                    .clamp(0.0, 1.0);
                let distance = (3.0_f32 - 2.2_f32 * interp).max(0.8_f32);
                Some(RearCameraScan {
                    obstacle_detected: true,
                    distance,
                })
            } else {
                // Path is clear — passenger may safely exit.
                Some(RearCameraScan {
                    obstacle_detected: false,
                    distance: 5.0,
                })
            }
        } else {
            None
        };

        (VehicleState { speed, gear }, rear_cam)
    }
}

impl Activity for RearCamera {
    fn id(&self) -> ActivityId {
        self.activity_id
    }

    #[instrument(name = "RearCamera startup")]
    fn startup(&mut self) -> Result<(), ActivityError> {
        use crate::dds_types::{RearCameraScan, VehicleState};
        use dust_dds::domain::domain_participant_factory::DomainParticipantFactory;
        use dust_dds::infrastructure::qos::{DataWriterQos, QosKind};
        use dust_dds::infrastructure::qos_policy::{
            DurabilityQosPolicy, DurabilityQosPolicyKind, HistoryQosPolicy, HistoryQosPolicyKind,
            ReliabilityQosPolicy, ReliabilityQosPolicyKind,
        };
        use dust_dds::infrastructure::status::NO_STATUS;
        use dust_dds::infrastructure::time::{Duration as DdsDuration, DurationKind};

        debug!("RearCamera: creating DDS participant on domain 100");

        let participant = DomainParticipantFactory::get_instance()
            .create_participant(100, QosKind::Default, None, NO_STATUS)
            .expect("RearCamera: failed to create DDS participant");

        let publisher = participant
            .create_publisher(QosKind::Default, None, NO_STATUS)
            .expect("RearCamera: failed to create DDS publisher");

        // QoS shared by both writers: reliable / transient-local / keep-last 5.
        // Reliable is required for TransientLocal replay to late-joining readers.
        let make_qos = || DataWriterQos {
            reliability: ReliabilityQosPolicy {
                kind: ReliabilityQosPolicyKind::Reliable,
                max_blocking_time: DurationKind::Finite(DdsDuration::new(0, 100_000_000)),
            },
            durability: DurabilityQosPolicy {
                kind: DurabilityQosPolicyKind::TransientLocal,
            },
            history: HistoryQosPolicy {
                kind: HistoryQosPolicyKind::KeepLast(5),
            },
            ..Default::default()
        };

        let vs_topic = participant
            .create_topic::<VehicleState>("vehicle/state", "VehicleState", QosKind::Default, None, NO_STATUS)
            .expect("RearCamera: failed to create vehicle/state topic");
        let vehicle_state_writer = publisher
            .create_datawriter::<VehicleState>(&vs_topic, QosKind::Specific(make_qos()), None, NO_STATUS)
            .expect("RearCamera: failed to create vehicle_state DataWriter");

        let rc_topic = participant
            .create_topic::<RearCameraScan>(
                "sensor/rear_camera",
                "RearCameraScan",
                QosKind::Default,
                None,
                NO_STATUS,
            )
            .expect("RearCamera: failed to create sensor/rear_camera topic");
        let rear_camera_writer = publisher
            .create_datawriter::<RearCameraScan>(
                &rc_topic,
                QosKind::Specific(make_qos()),
                None,
                NO_STATUS,
            )
            .expect("RearCamera: failed to create rear_camera DataWriter");

        // Give DDS discovery time to propagate before the first publish.
        std::thread::sleep(std::time::Duration::from_millis(200));

        self.dds = Some(DdsResources {
            vehicle_state_writer,
            rear_camera_writer,
            _participant: participant,
        });
        // Reset the demo cycle clock here so timing is relative to actual startup.
        self.cycle_start = std::time::Instant::now();

        debug!("RearCamera: DDS writers ready on topics vehicle/state and sensor/rear_camera");
        Ok(())
    }

    #[instrument(name = "RearCamera")]
    fn step(&mut self) -> Result<(), ActivityError> {
        debug!("Stepping RearCamera");
        let (vehicle_state, rear_cam) = self.compute_state();

        debug!(
            "RearCamera: speed={:.1} km/h  gear={}",
            vehicle_state.speed,
            if vehicle_state.gear == 0 { "P" } else { "D" }
        );

        if let Some(dds) = &self.dds {
            // Always publish vehicle state — fire and forget (UDP, non-blocking).
            dds.vehicle_state_writer.write(&vehicle_state, None).ok();

            // Publish rear-camera scan only when slow / parked.
            if let Some(scan) = rear_cam {
                debug!(
                    "RearCamera: publishing scan  obstacle={} distance={:.2}m",
                    scan.obstacle_detected, scan.distance
                );
                dds.rear_camera_writer.write(&scan, None).ok();
            }
        }

        Ok(())
    }

    #[instrument(name = "RearCamera shutdown")]
    fn shutdown(&mut self) -> Result<(), ActivityError> {
        debug!("Shutting down RearCamera activity {}", self.activity_id);
        // Drop writers before participant (field order in DdsResources ensures this).
        self.dds = None;
        Ok(())
    }
}

/// Generate a pseudo-random number in the specified range.
fn gen_random_in_range(range: Range<i64>) -> i64 {
    let rand = RandomState::new().build_hasher().finish();
    let rand = (rand % (i64::MAX as u64)) as i64;
    rand % (range.end - range.start + 1) + range.start
}

/// Random walk from `previous` with a probability of `change_prop` in a range of +/-`max_delta`
fn random_walk_float(previous: f64, change_prop: f64, max_delta: f64) -> f64 {
    if gen_random_in_range(0..100) as f64 / 100.0 < change_prop {
        const SCALE_FACTOR: f64 = 1000.0;

        // Scale delta to work in integers
        let scaled_max_delta = (max_delta * SCALE_FACTOR) as i64;
        let scaled_delta = gen_random_in_range(-scaled_max_delta..scaled_max_delta) as f64;

        return previous + (scaled_delta / SCALE_FACTOR);
    }

    previous
}

/// Random walk from `previous` with a probability of `change_prop` in a range of +/-`max_delta`
fn random_walk_integer(previous: usize, change_prop: f64, max_delta: usize) -> usize {
    let max_delta = max_delta as i64;

    if gen_random_in_range(0..100) as f64 / 100.0 < change_prop {
        let delta = gen_random_in_range(-max_delta..max_delta);

        return i64::max(0, previous as i64 + delta) as usize;
    }

    previous
}

fn load_delay_injection_config() -> Option<DelayInjectionConfig> {
    let every_cycles = std::env::var("MINI_ADAS_DELAY_EVERY_CYCLES")
        .ok()
        .and_then(|val| val.parse::<u64>().ok())?;
    let delay_ms = std::env::var("MINI_ADAS_DELAY_MS")
        .ok()
        .and_then(|val| val.parse::<u64>().ok())?;

    let target_activity_ids = std::env::var("MINI_ADAS_DELAY_ACTIVITY_IDS")
        .ok()
        .map(|value| {
            value
                .split(',')
                .filter_map(|entry| entry.trim().parse::<u64>().ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Some(DelayInjectionConfig {
        every_cycles,
        delay_ms,
        target_activity_ids,
    })
}

fn next_injected_delay_ms(activity_id: ActivityId) -> Option<u64> {
    static CONFIG: OnceLock<Option<DelayInjectionConfig>> = OnceLock::new();
    static COUNTERS: OnceLock<Mutex<HashMap<u64, u64>>> = OnceLock::new();

    let config = CONFIG.get_or_init(load_delay_injection_config).as_ref()?;
    let activity_id_raw = activity_id.id();

    if !config.target_activity_ids.is_empty() && !config.target_activity_ids.contains(&activity_id_raw) {
        return None;
    }

    let mut counters = COUNTERS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("delay injection counter mutex poisoned");

    let cycle_count = counters.entry(activity_id_raw).or_insert(0);
    *cycle_count += 1;

    if *cycle_count % config.every_cycles == 0 {
        Some(config.delay_ms)
    } else {
        None
    }
}

/// Sleep for a random amount of time plus an optional deterministic injected delay.
fn sleep_random(activity_id: ActivityId) {
    thread::sleep(Duration::from_millis(gen_random_in_range(SLEEP_RANGE) as u64));

    if let Some(delay_ms) = next_injected_delay_ms(activity_id) {
        info!(
            "Injecting deterministic delay of {} ms on activity {}",
            delay_ms,
            activity_id
        );
        thread::sleep(Duration::from_millis(delay_ms));
    }
}
