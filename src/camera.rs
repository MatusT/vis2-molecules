//!
//! Module that contains implementation(s) of camera(s).
//!

use nalgebra_glm as glm;

///
/// General trait for any implementation of camera.
///
pub trait Camera {
    fn eye(&self) -> glm::Vec3;
    fn set_speed(&mut self, speed: f32);
}

///
/// Rotation camera that always looks at the centre of the scene and rotates around It.
///
pub struct RotationCamera {
    pub eye: glm::Vec3,

    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,

    pub speed: f32,
    pub mouse_pressed: bool,
}

impl RotationCamera {
    ///
    /// Initializes rotation camera with some distance from the centre.
    ///
    pub fn new(distance: f32) -> RotationCamera {
        let camera = Self {
            eye: glm::vec3(0.0, 0.0, 0.0),

            yaw: -90.0,
            pitch: 0.0,
            distance,

            speed: 1.0,
            mouse_pressed: false,
        };

        camera
    }

    ///
    /// Returns direction vector pointing from the centre to a point on a sphere where the camera is located.
    ///
    pub fn direction_vector(&self) -> glm::Vec3 {
        let yaw = self.yaw.to_radians();
        let pitch = self.pitch.to_radians();

        glm::normalize(&glm::vec3(yaw.cos() * pitch.cos(), pitch.sin(), yaw.sin() * pitch.cos()))
    }
}

impl Camera for RotationCamera {
    fn eye(&self) -> glm::Vec3 {
        glm::vec3(self.eye[0], self.eye[1], self.eye[2])
    }

    fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }
}
